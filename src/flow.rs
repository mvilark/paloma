//! Flows: the steps of one task, run in order over a state the app owns.
//!
//! A step is plain app code that usually asks one or more prompts and can do anything
//! else in between (call a server, show a spinner, print a message). The flow numbers
//! the steps for the prompts' step counter (`01/05`), goes back a step on `Esc`, jumps
//! back to a named step with [`back_to`], and decides what stays on screen.
//!
//! ```rust,no_run
//! use paloma::{flow, Done, Flow, Input, Message};
//!
//! #[derive(Default)]
//! struct SignUp {
//!     email: String,
//!     name: String,
//! }
//!
//! let state = Flow::new(SignUp::default())
//!     .title("Sign up")
//!     .done(Done::Collapse)
//!     .step("email", |s| {
//!         // Going back keeps the earlier answer: the input opens with it.
//!         s.email = Input::new().prompt("Email").prefill(s.email.clone()).interact()?;
//!         Ok(())
//!     })
//!     .step("name", |s| {
//!         s.name = Input::new().prompt("Name").interact()?;
//!         if s.email.ends_with("@example.com") {
//!             return flow::back_to("email", Message::error("Use your own address"));
//!         }
//!         Ok(())
//!     })
//!     .run()?;
//! # Ok::<(), paloma::Error>(())
//! ```
//!
//! # What stays on screen
//!
//! paloma cannot read the screen back, so while a flow runs, every lasting line paloma
//! writes to a terminal (answered and kept prompts, messages, blocks, a task's step and
//! result lines) is recorded on the flow's ledger with the step that wrote it. Going
//! back erases the target step's output and everything after it, [`Flow::keep_last`]
//! erases older steps, and [`Flow::done`] erases the whole flow. Output written past
//! paloma (an app's own `println!`) is not recorded: inside a flow, print through
//! paloma, or call [`printed`] after printing.
use std::{
    io,
    sync::{Mutex, MutexGuard},
};

use console::{Term, TermTarget};

use crate::{
    options::Done,
    output::message::Message,
    prompts::row::MessageData,
    terminal::Stream,
    theme::{render::rows, FlowSummaryView, Theme},
    Error, Result,
};

/// The flow that runs now, if any.
static ACTIVE: Mutex<Option<Active>> = Mutex::new(None);

/// Locks the active flow, ignoring poison: a panic in a step must not stop the next
/// flow from running.
fn lock() -> MutexGuard<'static, Option<Active>> {
    ACTIVE.lock().unwrap_or_else(|e| e.into_inner())
}

/// What the running flow shares with the prompts and output of its steps.
struct Active {
    /// Index of the step that runs now.
    step: usize,
    /// The step counter `(number, total)`, or `None` when the counter is off.
    counter: Option<(usize, usize)>,
    /// Whether `Esc` in a prompt goes back.
    esc_back: bool,
    /// What answered prompts leave, unless they set their own `done`.
    step_done: Option<Done>,
    /// The message of a `back_to`, for the row of the next prompt that opens.
    pending: Option<MessageData>,
    /// Whether the current step has shown a prompt.
    asked: bool,
    /// What the steps printed.
    ledger: Ledger,
}

/// What a prompt reads from the running flow when it is shown.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Snapshot {
    /// The step counter, or `None` when the counter is off.
    pub(crate) counter: Option<(usize, usize)>,
    /// Whether `Esc` goes back.
    pub(crate) esc_back: bool,
    /// The flow's `step_done`.
    pub(crate) step_done: Option<Done>,
}

/// A copy of what prompts read from the running flow, or `None` outside a flow.
pub(crate) fn current() -> Option<Snapshot> {
    lock().as_ref().map(|a| Snapshot {
        counter: a.counter,
        esc_back: a.esc_back,
        step_done: a.step_done,
    })
}

/// The flow's `step_done`, outside a flow `None`.
pub(crate) fn step_done() -> Option<Done> {
    current().and_then(|s| s.step_done)
}

/// The step counter for a prompt shown now.
pub(crate) fn counter() -> Option<(usize, usize)> {
    current().and_then(|s| s.counter)
}

/// Whether `Esc` in `interact()` goes back: inside a flow with `esc_back` on.
pub(crate) fn esc_back() -> bool {
    current().is_some_and(|s| s.esc_back)
}

/// The answer of `interact()`: `None` (the prompt was left with `Esc`) is
/// [`Error::Back`] when `Esc` goes back, otherwise an error as before flows.
pub(crate) fn required<T>(answer: Result<Option<T>>, back: bool) -> Result<T> {
    match answer? {
        Some(value) => Ok(value),
        None if back => Err(Error::Back),
        None => Err(io::Error::other("Quit not allowed in this case").into()),
    }
}

/// The message a `back_to` left for the first prompt its step opens.
pub(crate) fn take_pending() -> Option<Message<'static>> {
    lock()
        .as_mut()
        .and_then(|a| a.pending.take())
        .map(MessageData::into_message)
}

/// The current step showed a prompt (presets and unattended answers do not count).
pub(crate) fn mark_asked() {
    if let Some(active) = lock().as_mut() {
        active.asked = true;
    }
}

/// Records `text`, just written to `stream` as whole lines (without the final line
/// feed), as output of the current step. Does nothing outside a flow.
pub(crate) fn record(stream: Stream, text: &str) {
    if let Some(active) = lock().as_mut() {
        let step = active.step;
        active.ledger.push(step, stream, text);
    }
}

/// [`record`]s `text`, just written to `term`, when `term` is a terminal (a redirected
/// stream never shows on screen).
pub(crate) fn record_on(term: &Term, text: &str) {
    if term.is_term() {
        let stream = match term.target() {
            TermTarget::Stdout => Stream::Stdout,
            _ => Stream::Stderr,
        };
        record(stream, text);
    }
}

/// Records `text` (one or more lines, already printed by the app) as output of the
/// current step, so going back, [`Flow::keep_last`] and [`Flow::done`] account for it.
/// Call it for text that reached the terminal, after printing it; one final line feed
/// is ignored. [`Flow::keep_last`] prints kept text again on stderr. Does nothing
/// outside a flow.
///
/// ```rust,no_run
/// println!("Using the staging server");
/// paloma::flow::printed("Using the staging server");
/// ```
pub fn printed(text: &str) {
    record(Stream::Stderr, text.strip_suffix('\n').unwrap_or(text));
}

/// Where a [`back_to`] jumps: the step's name and the message for its prompt's row.
/// Carried by [`Error::BackTo`] and handled by [`Flow::run`].
#[derive(Clone, Debug)]
pub struct BackTo {
    step: String,
    message: Option<MessageData>,
}

impl BackTo {
    /// The name of the step to go back to.
    pub fn step(&self) -> &str {
        &self.step
    }
}

/// Ends the current step and jumps back to the earlier step `step`, showing `message`
/// in the row of the first prompt that step opens. The current step's own name works
/// too: the step runs again.
///
/// ```rust,no_run
/// # use paloma::{flow, Message};
/// # fn step() -> paloma::Result {
/// return flow::back_to("email", Message::error("Wrong email or password"));
/// # }
/// ```
///
/// It returns [`Error::BackTo`], for the step to return with `?` or `return`;
/// [`Flow::run`] handles it. A name that matches no earlier step that ran makes `run`
/// return an `io::ErrorKind::InvalidInput` error.
pub fn back_to<T>(step: &str, message: impl Into<Option<Message<'static>>>) -> Result<T> {
    Err(Error::BackTo(BackTo {
        step: step.to_string(),
        message: message.into().as_ref().map(MessageData::new),
    }))
}

/// One recorded piece of output.
#[derive(Clone, Debug, PartialEq)]
struct Entry {
    /// The step that wrote it.
    step: usize,
    /// Where it was written.
    stream: Stream,
    /// The text, without the final line feed; may hold `\n`.
    text: String,
}

/// What the steps of a flow printed, in the order it reached the screen.
#[derive(Default, Debug)]
struct Ledger {
    entries: Vec<Entry>,
}

impl Ledger {
    fn push(&mut self, step: usize, stream: Stream, text: &str) {
        self.entries.push(Entry {
            step,
            stream,
            text: text.to_string(),
        });
    }

    /// The screen rows the output of steps `from..` takes on a terminal `width`
    /// columns wide.
    fn rows(&self, from: usize, width: usize) -> usize {
        self.entries
            .iter()
            .filter(|e| e.step >= from)
            .map(|e| rows(&e.text, width))
            .sum()
    }

    /// Removes the output of steps `from..`.
    fn drop_from(&mut self, from: usize) {
        self.entries.retain(|e| e.step < from);
    }

    /// The steps that have output, in order.
    fn steps(&self) -> Vec<usize> {
        let mut steps: Vec<usize> = Vec::new();
        for entry in &self.entries {
            if steps.last() != Some(&entry.step) {
                steps.push(entry.step);
            }
        }
        steps
    }

    /// The first step whose output `keep_last(n)` keeps, when older steps must go.
    fn keep_from(&self, n: usize) -> Option<usize> {
        let steps = self.steps();
        (steps.len() > n).then(|| steps.get(steps.len() - n).copied().unwrap_or(usize::MAX))
    }
}

/// The terminal flows erase on: stderr, or stdout when only stdout is a terminal.
fn screen() -> Option<Term> {
    [Term::stderr(), Term::stdout()]
        .into_iter()
        .find(Term::is_term)
}

/// Erases the last `rows` screen rows above the cursor (the cursor is at the start of
/// a fresh line). Rows that scrolled out of the screen cannot be reached.
fn erase_rows(term: &Term, rows: usize) {
    if rows == 0 {
        return;
    }
    let height = term.size().0 as usize;
    let _ = term.move_cursor_up(rows.min(height.max(1)));
    let _ = term.clear_to_end_of_screen();
    let _ = term.flush();
}

/// Erases what steps `from..` printed and forgets it.
fn erase_from(from: usize) {
    let mut guard = lock();
    let Some(active) = guard.as_mut() else {
        return;
    };
    if let Some(term) = screen() {
        let width = term.size().1 as usize;
        erase_rows(&term, active.ledger.rows(from, width));
    }
    active.ledger.drop_from(from);
}

/// Keeps only the output of the last `n` steps that printed something: erases the
/// flow's output and prints the kept part again.
fn keep_last(n: usize) {
    let mut guard = lock();
    let Some(active) = guard.as_mut() else {
        return;
    };
    let Some(from) = active.ledger.keep_from(n) else {
        return;
    };
    if let Some(term) = screen() {
        let width = term.size().1 as usize;
        erase_rows(&term, active.ledger.rows(0, width));
    }
    active.ledger.entries.retain(|e| e.step >= from);
    for entry in &active.ledger.entries {
        let term = entry.stream.term();
        let _ = term.write_line(&entry.text);
        let _ = term.flush();
    }
}

/// Registers a flow as the running one; dropping it (also while a panic unwinds)
/// clears it.
struct Registration;

impl Registration {
    fn new(esc_back: bool, step_done: Option<Done>) -> Result<Registration> {
        let mut guard = lock();
        if guard.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a flow is already running (flows cannot nest)",
            )
            .into());
        }
        *guard = Some(Active {
            step: 0,
            counter: None,
            esc_back,
            step_done,
            pending: None,
            asked: false,
            ledger: Ledger::default(),
        });
        Ok(Registration)
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        *lock() = None;
    }
}

/// The step counter of step `i`: its position among the steps that run, and how many
/// run. `runs[j]` says whether step `j` runs (it is unconditional, or its condition
/// holds for the state now).
fn numbering(runs: &[bool], i: usize) -> (usize, usize) {
    let count = |runs: &[bool]| runs.iter().filter(|r| **r).count();
    (count(&runs[..=i]), count(runs))
}

/// A step that finished: its index and whether it showed a prompt.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Ran {
    index: usize,
    asked: bool,
}

/// Where `Esc` goes back to: the position in `history` of the nearest earlier step
/// that asked something.
fn nearest_asked(history: &[Ran]) -> Option<usize> {
    history.iter().rposition(|r| r.asked)
}

/// Where `back_to(name)` goes: how many finished steps stay in the history, and the
/// index of the step to run. `None` when no earlier step that ran (nor the current
/// one) has that name.
fn back_to_target(
    history: &[Ran],
    names: &[&str],
    current: usize,
    name: &str,
) -> Option<(usize, usize)> {
    match history.iter().position(|r| names[r.index] == name) {
        Some(pos) => Some((pos, history[pos].index)),
        None => (names[current] == name).then_some((history.len(), current)),
    }
}

/// A step's code.
type StepFn<'a, S> = Box<dyn FnMut(&mut S) -> Result + 'a>;
/// A step's condition.
type Condition<'a, S> = Box<dyn Fn(&S) -> bool + 'a>;

struct Step<'a, S> {
    name: &'static str,
    condition: Option<Condition<'a, S>>,
    run: StepFn<'a, S>,
}

/// Runs the steps of one task in order over a state the app owns.
///
/// Each step is a closure over the state. `Ok(())` moves on to the next step;
/// `Err(Error::Back)` (what `interact()` returns for `Esc` inside a flow) goes back to
/// the nearest earlier step that asked something; [`back_to`] jumps back to a named
/// step; any other error ends the flow with it. Going back erases what the steps from
/// the target on printed, and the state keeps their values, so a step can prefill its
/// prompt. `^C` in a prompt ends the flow with [`Error::Cancelled`]. Presets and
/// unattended mode work per step, so a flow run unattended asks nothing.
///
/// See the [module docs](crate::flow) for an example and what is recorded.
pub struct Flow<'a, S> {
    state: S,
    steps: Vec<Step<'a, S>>,
    title: Option<String>,
    counter: Option<bool>,
    esc_back: Option<bool>,
    step_done: Option<Done>,
    keep_last: Option<usize>,
    done: Option<Done>,
    theme: Option<&'a dyn Theme>,
}

impl<'a, S> Flow<'a, S> {
    /// A flow over `state`; [`run`](Self::run) returns it.
    pub fn new(state: S) -> Self {
        Flow {
            state,
            steps: Vec::new(),
            title: None,
            counter: None,
            esc_back: None,
            step_done: None,
            keep_last: None,
            done: None,
            theme: None,
        }
    }

    /// A step: `f` runs app code and asks prompts, reading and writing the state.
    /// `Ok(())` moves on; `Err(Error::Back)` (Esc in a prompt) goes back;
    /// `Err` from [`back_to`] jumps back; any other error ends the flow with it.
    pub fn step(self, name: &'static str, f: impl FnMut(&mut S) -> Result + 'a) -> Self {
        self.push(name, None, Box::new(f))
    }

    /// A step that runs only while `condition` holds for the state. The step counter's
    /// total counts it only while it holds.
    pub fn step_if(
        self,
        condition: impl Fn(&S) -> bool + 'a,
        name: &'static str,
        f: impl FnMut(&mut S) -> Result + 'a,
    ) -> Self {
        self.push(name, Some(Box::new(condition)), Box::new(f))
    }

    fn push(
        mut self,
        name: &'static str,
        condition: Option<Condition<'a, S>>,
        run: StepFn<'a, S>,
    ) -> Self {
        self.steps.push(Step {
            name,
            condition,
            run,
        });
        self
    }

    /// The flow's name, used by the summary line of `done(Done::Collapse)`.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Whether prompts show the step counter. Default: on (or `Options::flows`).
    pub fn counter(mut self, on: bool) -> Self {
        self.counter = Some(on);
        self
    }

    /// Whether `Esc` goes back a step (and cancels the flow on the first one).
    /// Default: on (or `Options::flows`). Off: `Esc` does nothing in `interact()`.
    pub fn esc_back(mut self, on: bool) -> Self {
        self.esc_back = Some(on);
        self
    }

    /// What each answered prompt leaves, unless the prompt sets its own `done`.
    pub fn step_done(mut self, done: Done) -> Self {
        self.step_done = Some(done);
        self
    }

    /// Keeps only the output of the last `n` finished steps on screen while the flow
    /// runs. Default: all.
    pub fn keep_last(mut self, n: usize) -> Self {
        self.keep_last = Some(n);
        self
    }

    /// What the whole flow leaves when it finishes. Default `Done::Keep`.
    /// `Collapse` without a title acts like `Erase`.
    pub fn done(mut self, done: Done) -> Self {
        self.done = Some(done);
        self
    }

    /// Draws with `theme` instead of the app theme (the summary line; prompts keep
    /// their own theme settings).
    pub fn theme(mut self, theme: &'a dyn Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Whether step `i` runs for the state now.
    fn runs(&self, i: usize) -> bool {
        self.steps[i]
            .condition
            .as_ref()
            .is_none_or(|condition| condition(&self.state))
    }

    /// Runs the steps and returns the state. `Err(Error::Cancelled)` after `^C`, or
    /// after `Esc` on the first step. Another flow running (a flow inside a step) is an
    /// `io::ErrorKind::InvalidInput` error.
    pub fn run(mut self) -> Result<S> {
        let options = crate::options::current();
        let flows = &options.flows;
        let counter = self.counter.or(flows.counter).unwrap_or(true);
        let esc_back = self.esc_back.or(flows.esc_back).unwrap_or(true);
        let step_done = self.step_done.or(flows.step_done);
        let done = self.done.or(flows.done).unwrap_or(Done::Keep);
        let _registration = Registration::new(esc_back, step_done)?;

        let names: Vec<&str> = self.steps.iter().map(|s| s.name).collect();
        let mut history: Vec<Ran> = Vec::new();
        let mut next = 0;
        while let Some(i) = (next..self.steps.len()).find(|&i| self.runs(i)) {
            let runs: Vec<bool> = (0..self.steps.len()).map(|j| self.runs(j)).collect();
            if let Some(active) = lock().as_mut() {
                active.step = i;
                active.counter = counter.then(|| numbering(&runs, i));
                active.asked = false;
            }
            let result = (self.steps[i].run)(&mut self.state);
            let asked = match lock().as_mut() {
                Some(active) => {
                    active.pending = None;
                    active.asked
                }
                None => false,
            };
            match result {
                Ok(()) => {
                    history.push(Ran { index: i, asked });
                    next = i + 1;
                    if let Some(n) = self.keep_last {
                        keep_last(n);
                    }
                }
                Err(Error::Back) => match nearest_asked(&history) {
                    Some(pos) => {
                        let target = history[pos].index;
                        history.truncate(pos);
                        erase_from(target);
                        next = target;
                    }
                    None => {
                        if matches!(done, Done::Erase | Done::Collapse) {
                            erase_from(0);
                        }
                        return crate::error::cancelled();
                    }
                },
                Err(Error::BackTo(to)) => {
                    let Some((keep, target)) = back_to_target(&history, &names, i, &to.step) else {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("back_to: no earlier step named {:?}", to.step),
                        )
                        .into());
                    };
                    history.truncate(keep);
                    erase_from(target);
                    if let Some(active) = lock().as_mut() {
                        active.pending = to.message;
                    }
                    next = target;
                }
                Err(err) => return Err(err),
            }
        }

        match (done, &self.title) {
            (Done::Keep, _) => {}
            (Done::Collapse, Some(title)) => {
                erase_from(0);
                let view = FlowSummaryView::new(title, history.len());
                let term = Term::stderr();
                crate::output::block::emit(
                    &term,
                    Stream::Stderr,
                    self.theme,
                    None,
                    |f, theme, ctx| theme.format_flow_summary(f, ctx, &view),
                )?;
            }
            _ => erase_from(0),
        }
        Ok(self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ran(index: usize, asked: bool) -> Ran {
        Ran { index, asked }
    }

    #[test]
    fn the_total_grows_when_a_condition_becomes_true() {
        // Five steps, the fourth conditional: `01/04` while it does not run…
        let runs = [true, true, true, false, true];
        assert_eq!(numbering(&runs, 0), (1, 4));
        assert_eq!(numbering(&runs, 2), (3, 4));
        assert_eq!(numbering(&runs, 4), (4, 4));
        // …then `04/05` once its condition holds.
        let runs = [true, true, true, true, true];
        assert_eq!(numbering(&runs, 3), (4, 5));
        assert_eq!(numbering(&runs, 4), (5, 5));
        // A condition that turns false again shrinks the total.
        let runs = [true, false, true];
        assert_eq!(numbering(&runs, 2), (2, 2));
    }

    #[test]
    fn esc_goes_back_to_the_nearest_step_that_asked() {
        let history = [ran(0, true), ran(1, false), ran(2, true), ran(3, false)];
        assert_eq!(nearest_asked(&history), Some(2));
        assert_eq!(nearest_asked(&history[..2]), Some(0));
        // Only presets before: nowhere to go, the flow cancels.
        assert_eq!(nearest_asked(&[ran(0, false), ran(1, false)]), None);
        assert_eq!(nearest_asked(&[]), None);
    }

    #[test]
    fn back_to_finds_earlier_steps_only() {
        let names = ["email", "password", "port", "sign in"];
        let history = [ran(0, true), ran(1, true), ran(2, false)];
        assert_eq!(back_to_target(&history, &names, 3, "email"), Some((0, 0)));
        assert_eq!(
            back_to_target(&history, &names, 3, "password"),
            Some((1, 1))
        );
        // The current step runs again.
        assert_eq!(back_to_target(&history, &names, 3, "sign in"), Some((3, 3)));
        // A later step or an unknown name is not a target.
        assert_eq!(back_to_target(&history[..1], &names, 1, "port"), None);
        assert_eq!(back_to_target(&history, &names, 3, "nope"), None);
        // A step that was skipped (its condition was false) is not a target either.
        let history = [ran(0, true), ran(1, true)];
        assert_eq!(back_to_target(&history, &names, 3, "port"), None);
    }

    #[test]
    fn the_ledger_counts_wrapped_rows_on_both_streams() {
        let mut ledger = Ledger::default();
        ledger.push(0, Stream::Stderr, "? Email: ada@example.com");
        ledger.push(1, Stream::Stdout, &"x".repeat(25));
        ledger.push(1, Stream::Stderr, "\n# Setup");
        ledger.push(2, Stream::Stderr, "\x1b[32m✔\x1b[0m done\nnext");
        // 10 columns: 3 rows, 3 rows, 2 rows (blank line and title), 2 rows.
        assert_eq!(ledger.rows(0, 10), 10);
        assert_eq!(ledger.rows(1, 10), 7);
        assert_eq!(ledger.rows(2, 10), 2);
        assert_eq!(ledger.rows(3, 10), 0);
        // Wider: no wrapping.
        assert_eq!(ledger.rows(0, 100), 6);
        assert_eq!(ledger.steps(), vec![0, 1, 2]);
        ledger.drop_from(1);
        assert_eq!(ledger.steps(), vec![0]);
        assert_eq!(ledger.rows(0, 100), 1);
    }

    #[test]
    fn keep_last_keeps_the_newest_steps_with_output() {
        let mut ledger = Ledger::default();
        for step in [0, 2, 2, 3] {
            ledger.push(step, Stream::Stderr, "line");
        }
        assert_eq!(ledger.keep_from(3), None);
        assert_eq!(ledger.keep_from(2), Some(2));
        assert_eq!(ledger.keep_from(1), Some(3));
        assert_eq!(ledger.keep_from(0), Some(usize::MAX));
        assert_eq!(Ledger::default().keep_from(0), None);
    }

    #[test]
    fn interact_outside_a_flow_still_refuses_esc() {
        assert!(matches!(required(Ok(Some(1)), false), Ok(1)));
        assert!(matches!(required::<u8>(Ok(None), true), Err(Error::Back)));
        assert!(matches!(required::<u8>(Ok(None), false), Err(Error::IO(_))));
        assert!(matches!(
            required::<u8>(Err(Error::Cancelled), true),
            Err(Error::Cancelled)
        ));
    }

    #[test]
    fn back_to_carries_its_message_as_data() {
        let Err(Error::BackTo(to)) = back_to::<()>(
            "email",
            Message::error("Wrong").nest(Message::hint("Try again")),
        ) else {
            panic!("not a BackTo");
        };
        assert_eq!(to.step(), "email");
        let message = to.message.unwrap().into_message();
        assert_eq!(message.text(), "Wrong");
        assert_eq!(message.children()[0].text(), "Try again");
        fn send_sync<T: Send + Sync>(_: &T) {}
        send_sync(&back_to::<()>("x", None).unwrap_err());
    }
}
