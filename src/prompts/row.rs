//! The message row under a prompt: its description, its `on_change` closure and what
//! it shows right now, the background checks of `change_delay`, and the key source a
//! prompt reads from ([`Keys`]).
#[cfg(any(unix, windows))]
use std::{
    sync::{atomic::AtomicBool, atomic::Ordering, mpsc},
    thread,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use console::{Key, Term};

#[cfg(any(unix, windows))]
use crate::keys::{read_key_timeout, RawMode};
use crate::{
    keys::TIMED_INPUT,
    output::message::{Message, Status},
    theme::MessageSource,
    Result,
};

/// An `on_change` closure: the message for the prompt's current value, or `None` for
/// the description.
pub(crate) type OnChange<'a, V> = Arc<Mutex<dyn FnMut(&V) -> Option<Message<'static>> + Send + 'a>>;

/// How long a prompt with a running background check waits for a key before it looks
/// for the check's result.
#[cfg(any(unix, windows))]
const RESULT_POLL: Duration = Duration::from_millis(100);

/// A message as plain data, so it can cross threads: [`Message`] holds a theme
/// reference, which is not `Send`. Its stream and theme are not kept (the row is drawn
/// by the prompt's theme).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MessageData {
    status: Status,
    text: String,
    children: Vec<MessageData>,
}

impl MessageData {
    /// The data of `message` and its nested messages.
    pub(crate) fn new(message: &Message) -> Self {
        MessageData {
            status: message.status(),
            text: message.text().to_string(),
            children: message.children().iter().map(MessageData::new).collect(),
        }
    }

    /// The message again, with its nested messages.
    pub(crate) fn into_message(self) -> Message<'static> {
        self.children
            .into_iter()
            .fold(Message::new(self.status, self.text), |message, child| {
                message.nest(child.into_message())
            })
    }
}

/// What a prompt's row receives (`str`, `[usize]`, `usize`, `bool`), with the owned
/// form a background check gets on its thread: like `ToOwned`, but text is wiped when
/// dropped (it may be a password).
pub(crate) trait RowValue {
    /// The value as a background check gets it.
    type Owned: Send;
    /// Copies the value for a background check.
    fn to_owned_value(&self) -> Self::Owned;
    /// The value inside its owned form.
    fn borrow_owned(owned: &Self::Owned) -> &Self;
}

/// Text sent to a background check, wiped when dropped (with the `password` feature,
/// which brings `zeroize`).
pub(crate) struct Text(String);

impl Drop for Text {
    fn drop(&mut self) {
        #[cfg(feature = "password")]
        zeroize::Zeroize::zeroize(&mut self.0);
    }
}

impl RowValue for str {
    type Owned = Text;
    fn to_owned_value(&self) -> Text {
        Text(self.to_string())
    }
    fn borrow_owned(owned: &Text) -> &str {
        &owned.0
    }
}

impl RowValue for [usize] {
    type Owned = Vec<usize>;
    fn to_owned_value(&self) -> Vec<usize> {
        self.to_vec()
    }
    fn borrow_owned(owned: &Vec<usize>) -> &[usize] {
        owned
    }
}

macro_rules! copy_values {
    ($($ty:ty),*) => {$(
        impl RowValue for $ty {
            type Owned = $ty;
            fn to_owned_value(&self) -> $ty {
                *self
            }
            fn borrow_owned(owned: &$ty) -> &$ty {
                owned
            }
        }
    )*};
}
copy_values!(usize, bool);

/// What the row shows, apart from the description.
#[derive(Clone)]
enum RowState {
    /// The description, if there is one.
    Description,
    /// What `on_change` returned for the current value.
    Change(Message<'static>),
    /// A validation error, until the value changes.
    Invalid(Message<'static>),
}

/// The row under a prompt whose value is a `V`. The description lives in the prompt's
/// [`PromptBase`](crate::prompts::base::PromptBase), shared by every prompt.
///
/// With a `delay` (`change_delay`), `on_change` runs in the background: a change only
/// records the value and when it is due ([`changed`](Self::changed)), the prompt's
/// [`Keys`] hands a due value to a worker thread ([`take_due`](Self::take_due)), and
/// the result is applied only if no newer change came in the meantime
/// ([`finished`](Self::finished)). Each change starts a new generation; a result
/// carries the generation of the value it was computed for.
pub(crate) struct Row<'a, V: ?Sized + RowValue> {
    on_change: Option<OnChange<'a, V>>,
    state: RowState,
    /// How long the value must stay unchanged before a background check runs.
    delay: Option<Duration>,
    /// Shown while a background check or the validators run.
    pending: Option<Message<'static>>,
    /// Counts the changes; a result for an older generation is dropped.
    generation: u64,
    /// The latest value and when its check is due, until it is sent.
    due: Option<(Instant, V::Owned)>,
    /// A check of the latest value is running.
    running: bool,
    /// A submit drew the pending row and the validators run next.
    validating: bool,
}

impl<V: ?Sized + RowValue> Clone for Row<'_, V> {
    /// Copies the settings and what the row shows; a background check in progress
    /// belongs to the prompt that runs it and is not copied.
    fn clone(&self) -> Self {
        Row {
            on_change: self.on_change.clone(),
            state: self.state.clone(),
            delay: self.delay,
            pending: self.pending.clone(),
            generation: self.generation,
            due: None,
            running: false,
            validating: false,
        }
    }
}

impl<V: ?Sized + RowValue> Default for Row<'_, V> {
    fn default() -> Self {
        Row {
            on_change: None,
            state: RowState::Description,
            delay: None,
            pending: None,
            generation: 0,
            due: None,
            running: false,
            validating: false,
        }
    }
}

impl<'a, V: ?Sized + RowValue> Row<'a, V> {
    /// Sets the `on_change` closure, replacing an earlier one.
    pub(crate) fn on_change(&mut self, f: impl FnMut(&V) -> Option<Message<'static>> + Send + 'a) {
        self.on_change = Some(Arc::new(Mutex::new(f)));
    }

    /// Runs `on_change` in the background once the value has not changed for `delay`.
    pub(crate) fn set_delay(&mut self, delay: Duration) {
        self.delay = Some(delay);
    }

    /// What the row shows while a background check or the validators run.
    pub(crate) fn set_pending(&mut self, message: Message<'static>) {
        self.pending = Some(message);
    }

    /// The closure a background worker runs, when checks run in the background: a
    /// delay and a closure are set, and keys can be read with a timeout here.
    pub(crate) fn worker(&self) -> Option<OnChange<'a, V>> {
        match (&self.on_change, self.delay) {
            (Some(f), Some(_)) if TIMED_INPUT => Some(Arc::clone(f)),
            _ => None,
        }
    }

    /// The value changed: asks `on_change` what to show, or goes back to the
    /// description. A validation error goes away.
    ///
    /// With a delay, the closure is not called here: the value is recorded and its
    /// check is due once the delay has passed without another change. The row keeps
    /// what it showed meanwhile (a validation error still goes). Where keys cannot be
    /// read with a timeout, the check never runs.
    pub(crate) fn changed(&mut self, value: &V) {
        self.changed_at(value, Instant::now());
    }

    fn changed_at(&mut self, value: &V, now: Instant) {
        if let (Some(delay), Some(_)) = (self.delay, &self.on_change) {
            if let RowState::Invalid(_) = self.state {
                self.state = RowState::Description;
            }
            self.generation = self.generation.wrapping_add(1);
            // A check that runs now is for an older value: its result will be dropped.
            self.running = false;
            if TIMED_INPUT {
                self.due = Some((now + delay, value.to_owned_value()));
            }
            return;
        }
        let message = match &self.on_change {
            Some(f) => f.lock().unwrap_or_else(|e| e.into_inner())(value),
            None => None,
        };
        self.state = match message {
            Some(message) => RowState::Change(message),
            None => RowState::Description,
        };
    }

    /// How long until the next check is due, if one is waiting.
    #[cfg_attr(not(any(unix, windows)), allow(dead_code))]
    fn until_due(&self, now: Instant) -> Option<Duration> {
        self.due
            .as_ref()
            .map(|(at, _)| at.saturating_duration_since(now))
    }

    /// The check that is due at `now`, as `(generation, value)`: the row now counts it
    /// as running.
    #[cfg_attr(not(any(unix, windows)), allow(dead_code))]
    fn take_due(&mut self, now: Instant) -> Option<(u64, V::Owned)> {
        match &self.due {
            Some((at, _)) if *at <= now => {
                let (_, value) = self.due.take()?;
                self.running = true;
                Some((self.generation, value))
            }
            _ => None,
        }
    }

    /// A background check of generation `generation` returned `result`. Returns
    /// whether the row changed: a result for an older value is dropped.
    #[cfg_attr(not(any(unix, windows)), allow(dead_code))]
    fn finished(&mut self, generation: u64, result: Option<MessageData>) -> bool {
        if generation != self.generation || !self.running {
            return false;
        }
        self.running = false;
        self.state = match result {
            Some(data) => RowState::Change(data.into_message()),
            None => RowState::Description,
        };
        true
    }

    /// Whether the row shows the pending message now.
    fn shows_pending(&self) -> bool {
        self.pending.is_some() && (self.running || self.validating)
    }

    /// A submit: when `validates` (validators will run) and a pending message is set,
    /// returns true and the prompt draws once more (showing the pending message) before
    /// running them; the next call, for that same submit, returns false.
    fn defer_submit(&mut self, validates: bool) -> bool {
        if self.validating {
            self.validating = false;
            false
        } else {
            self.validating = validates && self.pending.is_some();
            self.validating
        }
    }

    /// A validation failed: `message` shows until the value changes.
    pub(crate) fn invalid(&mut self, message: Message<'static>) {
        self.state = RowState::Invalid(message);
    }

    /// The row shows a validation error.
    pub(crate) fn is_invalid(&self) -> bool {
        matches!(self.state, RowState::Invalid(_)) && !self.shows_pending()
    }

    /// What the row shows and where it comes from, or `None` when it is empty.
    pub(crate) fn current<'d>(
        &'d self,
        description: Option<&'d Message<'static>>,
    ) -> Option<(&'d Message<'static>, MessageSource)> {
        if let (Some(pending), true) = (&self.pending, self.shows_pending()) {
            return Some((pending, MessageSource::Change));
        }
        match &self.state {
            RowState::Description => description.map(|d| (d, MessageSource::Description)),
            RowState::Change(message) => Some((message, MessageSource::Change)),
            RowState::Invalid(message) => Some((message, MessageSource::Validation)),
        }
    }
}

/// Where a prompt reads its keys from. Without a background check it reads them with
/// `console` as usual. With one (see [`Row`]) it holds the terminal in raw mode, reads
/// with a timeout, sends due checks to a worker thread and applies their results
/// between keys. It also replays the `Enter` of a submit that first draws the pending
/// row ([`defer_submit`](Self::defer_submit)).
pub(crate) struct Keys<'s, V: ?Sized + RowValue> {
    replay: Option<Key>,
    #[cfg(any(unix, windows))]
    background: Option<Background<'s, V>>,
    #[cfg(not(any(unix, windows)))]
    background: Option<std::marker::PhantomData<&'s V>>,
}

/// The worker thread of a prompt's background checks, and the raw mode the prompt
/// holds while it lives.
#[cfg(any(unix, windows))]
struct Background<'s, V: ?Sized + RowValue> {
    /// `(generation, value)` to check.
    jobs: mpsc::Sender<(u64, V::Owned)>,
    /// `(generation, message)`: what `on_change` returned, as plain data.
    results: mpsc::Receiver<(u64, Option<MessageData>)>,
    worker: Option<thread::ScopedJoinHandle<'s, ()>>,
    /// Set when the prompt ends: the worker runs no further check.
    stop: &'s AtomicBool,
    _raw: RawMode,
}

#[cfg(any(unix, windows))]
impl<V: ?Sized + RowValue> Drop for Background<'_, V> {
    fn drop(&mut self) {
        // Dropping `jobs` next ends the worker once a running check returns.
        self.stop.store(true, Ordering::SeqCst);
    }
}

impl<V: ?Sized + RowValue> Keys<'_, V> {
    /// Runs a prompt's loop `f` with its keys: with a background worker for `worker`
    /// (from [`Row::worker`]), or plain keys for `None`.
    ///
    /// The worker is a scoped thread, so the closure needs no `'static` bound; the
    /// prompt returns once a check that is running has finished.
    pub(crate) fn run<'a, R>(
        worker: Option<OnChange<'a, V>>,
        f: impl FnOnce(&mut Keys<'_, V>) -> Result<R>,
    ) -> Result<R> {
        #[cfg(any(unix, windows))]
        if let Some(on_change) = worker {
            let raw = RawMode::enter()?;
            let stop = AtomicBool::new(false);
            return thread::scope(|s| {
                let (jobs, job_rx) = mpsc::channel::<(u64, V::Owned)>();
                let (result_tx, results) = mpsc::channel();
                let stop = &stop;
                let worker = s.spawn(move || {
                    while let Ok(mut job) = job_rx.recv() {
                        // Only the latest value is worth checking.
                        while let Ok(newer) = job_rx.try_recv() {
                            job = newer;
                        }
                        if stop.load(Ordering::SeqCst) {
                            continue;
                        }
                        let message = on_change.lock().unwrap_or_else(|e| e.into_inner())(
                            V::borrow_owned(&job.1),
                        );
                        // Only plain data crosses threads.
                        let data = message.as_ref().map(MessageData::new);
                        if result_tx.send((job.0, data)).is_err() {
                            break;
                        }
                    }
                });
                let mut keys = Keys {
                    replay: None,
                    background: Some(Background {
                        jobs,
                        results,
                        worker: Some(worker),
                        stop,
                        _raw: raw,
                    }),
                };
                f(&mut keys)
            });
        }
        #[cfg(not(any(unix, windows)))]
        drop(worker);
        f(&mut Keys {
            replay: None,
            background: None,
        })
    }

    /// The next key, or `None` when the row changed without a key (a background check
    /// started with a pending message, or a result came in) and the prompt must draw
    /// again.
    pub(crate) fn next(&mut self, term: &Term, row: &mut Row<'_, V>) -> Result<Option<Key>> {
        if let Some(key) = self.replay.take() {
            return Ok(Some(key));
        }
        #[cfg(any(unix, windows))]
        if let Some(background) = &mut self.background {
            return background.next(term, row);
        }
        #[cfg(not(any(unix, windows)))]
        let _ = row;
        Ok(Some(term.read_key_raw()?))
    }

    /// Called on a submit before the validators run: when `validates` and the row has
    /// a pending message, the submit waits one frame (the prompt draws the pending
    /// row, then gets the same `Enter` again) and this returns true. The prompt then
    /// goes back to the top of its loop.
    pub(crate) fn defer_submit(&mut self, row: &mut Row<'_, V>, validates: bool) -> bool {
        let defer = row.defer_submit(validates);
        if defer {
            self.replay = Some(Key::Enter);
        }
        defer
    }
}

#[cfg(any(unix, windows))]
impl<V: ?Sized + RowValue> Background<'_, V> {
    fn next(&mut self, term: &Term, row: &mut Row<'_, V>) -> Result<Option<Key>> {
        loop {
            let mut redraw = false;
            loop {
                match self.results.try_recv() {
                    Ok((generation, result)) => redraw |= row.finished(generation, result),
                    Err(mpsc::TryRecvError::Empty) => break,
                    // The worker ended early: `on_change` panicked. Panic here too.
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if let Some(Err(panic)) = self.worker.take().map(|w| w.join()) {
                            std::panic::resume_unwind(panic);
                        }
                        break;
                    }
                }
            }
            let now = Instant::now();
            if let Some(job) = row.take_due(now) {
                redraw |= row.shows_pending();
                let _ = self.jobs.send(job);
            }
            if redraw {
                return Ok(None);
            }
            let wait = match (row.until_due(now), row.running) {
                (Some(due), _) => due.min(RESULT_POLL),
                (None, true) => RESULT_POLL,
                // Nothing to wait for but a key.
                (None, false) => return Ok(Some(term.read_key_raw()?)),
            };
            if let Some(key) = read_key_timeout(term, wait)? {
                return Ok(Some(key));
            }
        }
    }
}

/// A validator: `Some(error)` when the value is refused.
type Check<'a, V> = Arc<Mutex<dyn FnMut(&V) -> Option<Message<'static>> + 'a>>;

/// The validators of a prompt, run in the order they were added; the first error wins.
pub(crate) struct Validators<'a, V: ?Sized>(Vec<Check<'a, V>>);

impl<V: ?Sized> Clone for Validators<'_, V> {
    fn clone(&self) -> Self {
        Validators(self.0.clone())
    }
}

impl<V: ?Sized> Default for Validators<'_, V> {
    fn default() -> Self {
        Validators(Vec::new())
    }
}

impl<'a, V: ?Sized> Validators<'a, V> {
    /// Adds a validator after the others.
    pub(crate) fn push(&mut self, f: impl FnMut(&V) -> Option<Message<'static>> + 'a) {
        self.0.push(Arc::new(Mutex::new(f)));
    }

    /// Whether no validator was added.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The first error any validator returns for `value`.
    pub(crate) fn check(&self, value: &V) -> Option<Message<'static>> {
        self.0
            .iter()
            .find_map(|f| f.lock().unwrap_or_else(|e| e.into_inner())(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IntoMessage;

    fn shown(
        row: &Row<str>,
        description: Option<&Message<'static>>,
    ) -> Option<(String, MessageSource)> {
        row.current(description)
            .map(|(m, source)| (m.text().to_string(), source))
    }

    #[test]
    fn the_row_shows_the_latest_source() {
        let description = Message::description("3 to 16 letters");
        let mut row: Row<str> = Row::default();
        row.on_change(|text: &str| (text.len() < 3).then(|| Message::error("Too short")));

        // Description first.
        let d = Some(&description);
        assert_eq!(
            shown(&row, d),
            Some(("3 to 16 letters".into(), MessageSource::Description))
        );
        // A change replaces it, `None` brings it back.
        row.changed("ab");
        assert_eq!(
            shown(&row, d),
            Some(("Too short".into(), MessageSource::Change))
        );
        row.changed("abc");
        assert_eq!(
            shown(&row, d),
            Some(("3 to 16 letters".into(), MessageSource::Description))
        );
        // An error replaces it until the next change.
        row.invalid(Message::error("Taken"));
        assert!(row.is_invalid());
        assert_eq!(
            shown(&row, d),
            Some(("Taken".into(), MessageSource::Validation))
        );
        row.changed("abcd");
        assert!(!row.is_invalid());
        assert_eq!(
            shown(&row, d),
            Some(("3 to 16 letters".into(), MessageSource::Description))
        );
    }

    #[test]
    fn without_on_change_or_description_the_row_is_empty() {
        let mut row: Row<[usize]> = Row::default();
        assert!(row.current(None).is_none());
        row.changed(&[1, 2]);
        assert!(row.current(None).is_none());
        row.invalid(Message::error("Pick one"));
        let (message, source) = row.current(None).unwrap();
        assert_eq!(
            (message.status(), source),
            (Status::Error, MessageSource::Validation)
        );
    }

    #[test]
    fn the_last_on_change_wins_and_clones_share_it() {
        let mut row: Row<usize> = Row::default();
        row.on_change(|_| Some(Message::hint("first")));
        row.on_change(|i| Some(Message::hint(format!("item {i}"))));
        let mut copy = row.clone();
        copy.changed(&2);
        assert_eq!(copy.current(None).unwrap().0.text(), "item 2");
        assert!(row.current(None).is_none());
    }

    /// A row with a background check: `on_change` must never run on this thread.
    fn delayed() -> Row<'static, str> {
        let mut row: Row<str> = Row::default();
        row.on_change(|_: &str| panic!("a delayed check runs on the input thread"));
        row.set_delay(Duration::from_millis(300));
        row
    }

    fn data(text: &str) -> Option<MessageData> {
        Some(MessageData::new(&Message::success(text)))
    }

    #[test]
    fn a_delayed_check_is_due_once_the_value_rests() {
        let (mut row, t0) = (delayed(), Instant::now());
        let ms = Duration::from_millis;
        assert!(row.worker().is_some());
        row.changed_at("a", t0);
        row.changed_at("ab", t0 + ms(50));
        // Not yet due: 300 ms after the last change.
        assert_eq!(row.until_due(t0 + ms(100)), Some(ms(250)));
        assert!(row.take_due(t0 + ms(300)).is_none());
        let (generation, value) = row.take_due(t0 + ms(350)).unwrap();
        assert_eq!(str::borrow_owned(&value), "ab");
        assert!(row.running && row.until_due(t0 + ms(350)).is_none());
        // Sent once.
        assert!(row.take_due(t0 + ms(400)).is_none());
        assert!(row.finished(generation, data("ab is free")));
        assert!(!row.running);
        assert_eq!(
            shown(&row, None),
            Some(("ab is free".into(), MessageSource::Change))
        );
    }

    #[test]
    fn a_result_for_an_older_value_is_dropped() {
        let (mut row, t0) = (delayed(), Instant::now());
        let ms = Duration::from_millis;
        let description = Message::description("pick a name");
        let d = Some(&description);
        row.changed_at("ad", t0);
        let (old, _) = row.take_due(t0 + ms(300)).unwrap();
        // The user types again while that check runs.
        row.changed_at("ada", t0 + ms(400));
        assert!(!row.running);
        assert!(!row.finished(old, data("ad is free")));
        assert_eq!(
            shown(&row, d),
            Some(("pick a name".into(), MessageSource::Description))
        );
        // The latest one is applied; `None` brings the description back.
        let (latest, _) = row.take_due(t0 + ms(700)).unwrap();
        assert!(latest != old);
        assert!(row.finished(latest, data("ada is taken")));
        assert_eq!(
            shown(&row, d),
            Some(("ada is taken".into(), MessageSource::Change))
        );
        row.changed_at("adam", t0 + ms(800));
        let (next, _) = row.take_due(t0 + ms(1100)).unwrap();
        assert!(row.finished(next, None));
        assert_eq!(
            shown(&row, d),
            Some(("pick a name".into(), MessageSource::Description))
        );
    }

    #[test]
    fn pending_shows_while_the_check_runs_and_otherwise_the_row_keeps_what_it_had() {
        let (mut row, t0) = (delayed(), Instant::now());
        let ms = Duration::from_millis;
        row.changed_at("a", t0);
        let (generation, _) = row.take_due(t0 + ms(300)).unwrap();
        assert!(row.finished(generation, data("a is free")));
        // Without `pending`, the row keeps the last result while the next check runs.
        row.changed_at("ab", t0 + ms(400));
        assert_eq!(shown(&row, None).unwrap().0, "a is free");
        let (generation, _) = row.take_due(t0 + ms(700)).unwrap();
        assert!(!row.shows_pending());
        assert_eq!(shown(&row, None).unwrap().0, "a is free");
        assert!(row.finished(generation, None));

        // With it, the pending message shows from the moment the check is sent.
        row.set_pending(Message::info("Checking…"));
        row.changed_at("abc", t0 + ms(800));
        assert!(!row.shows_pending());
        let (generation, _) = row.take_due(t0 + ms(1100)).unwrap();
        assert!(row.shows_pending());
        let (message, source) = row.current(None).unwrap();
        assert_eq!(
            (message.status(), message.text(), source),
            (Status::Info, "Checking…", MessageSource::Change)
        );
        assert!(row.finished(generation, data("abc is free")));
        assert_eq!(shown(&row, None).unwrap().0, "abc is free");
    }

    #[test]
    fn a_delayed_change_clears_a_validation_error() {
        let (mut row, t0) = (delayed(), Instant::now());
        row.invalid(Message::error("Taken"));
        assert!(row.is_invalid());
        row.changed_at("x", t0);
        assert!(!row.is_invalid());
        assert!(row.current(None).is_none());
    }

    #[test]
    fn without_a_delay_or_a_closure_nothing_runs_in_the_background() {
        let mut row: Row<str> = Row::default();
        row.set_delay(Duration::from_millis(300));
        assert!(row.worker().is_none());
        row.changed("a");
        assert!(row.due.is_none());

        let mut row: Row<usize> = Row::default();
        row.on_change(|i| Some(Message::hint(format!("item {i}"))));
        assert!(row.worker().is_none());
        row.changed(&3);
        assert!(row.due.is_none());
        assert_eq!(row.current(None).unwrap().0.text(), "item 3");
    }

    #[test]
    fn a_submit_shows_pending_for_one_frame_before_the_validators() {
        let mut row: Row<usize> = Row::default();
        // No pending message, or nothing to validate: no extra frame.
        assert!(!row.defer_submit(true));
        row.set_pending("Checking".into_message(Status::Info));
        assert!(!row.defer_submit(false));
        // The first call defers and shows pending, the replayed one goes on.
        assert!(row.defer_submit(true));
        assert_eq!(row.current(None).unwrap().0.text(), "Checking");
        assert!(!row.defer_submit(true));
        assert!(row.current(None).is_none());
        // An error shown before is hidden while pending shows.
        row.invalid(Message::error("Pick another"));
        assert!(row.defer_submit(true));
        assert!(!row.is_invalid());
        assert!(!row.defer_submit(true));
        assert!(row.is_invalid());
    }

    #[test]
    fn message_data_round_trips_nesting() {
        let message = Message::error("Password too weak")
            .nest(Message::hint("At least 8 characters").nest(Message::info("deeper")))
            .nest(Message::hint("One digit"));
        let data = MessageData::new(&message);
        let back = data.clone().into_message();
        assert_eq!(MessageData::new(&back), data);
        assert_eq!(
            (back.status(), back.text()),
            (Status::Error, "Password too weak")
        );
        let children: Vec<_> = back.children().iter().map(|c| c.text()).collect();
        assert_eq!(children, ["At least 8 characters", "One digit"]);
        assert_eq!(back.children()[0].children()[0].text(), "deeper");
        assert_eq!(back.children()[0].children()[0].status(), Status::Info);
    }

    #[test]
    fn validators_run_in_order_and_the_first_error_wins() {
        let mut v: Validators<usize> = Validators::default();
        v.push(|&i| (i == 0).then(|| Message::error("zero")));
        v.push(|&i| (i < 2).then(|| Message::error("small")));
        assert_eq!(v.check(&0).unwrap().text(), "zero");
        assert_eq!(v.check(&1).unwrap().text(), "small");
        assert!(v.check(&2).is_none());
    }
}
