//! The live region: one line that is redrawn from a background thread while work runs.
//!
//! At most one region exists at a time (several at once is a later phase). It draws a
//! task or a progress bar (its [`Body`]) and owns the hidden cursor: [`Region::finish`],
//! [`restore`] and the drop of a [`RunningTask`](crate::RunningTask) or
//! [`RunningProgress`](crate::RunningProgress) all show the cursor again. Every write to the
//! terminal happens while holding the frame lock, so a step line, a redraw and a
//! result line never interleave.
use std::{
    io,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use console::Term;

use crate::{
    live::{
        progress::{self, ProgressState, ProgressView},
        task::{TaskState, TaskStepView, TaskView},
    },
    output::message::Status,
    terminal::Stream,
    theme::{Context, LiveKind, Theme, Tokens},
};

/// The region on screen, if any.
static ACTIVE: Mutex<Option<Arc<Region>>> = Mutex::new(None);

/// Locks `mutex`, ignoring poison: a panic in a theme hook must not stop the cursor
/// from coming back.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// What a region draws, and the state the redraw thread advances.
pub(crate) enum Body {
    /// A task: a label and a spinner.
    Task {
        /// The running label.
        label: String,
        /// Index of the spinner frame on screen.
        counter: usize,
    },
    /// A progress bar: a label and a value out of a total.
    Progress {
        /// The running label.
        label: String,
        /// The real value, never above `total` and never decreasing.
        value: u64,
        /// The value on screen: eases toward `value` and never decreases.
        shown: f64,
        /// The value that means done.
        total: u64,
    },
}

impl Body {
    /// A task body: the running label and the spinner at its first frame.
    pub(crate) fn task(label: String) -> Self {
        Body::Task { label, counter: 0 }
    }

    /// A progress bar body at 0.
    pub(crate) fn progress(label: String, total: u64) -> Self {
        Body::Progress {
            label,
            value: 0,
            shown: 0.0,
            total,
        }
    }
}

/// What the redraw thread draws.
struct Frame {
    body: Body,
}

/// What a region leaves on screen when it ends.
pub(crate) enum Leave<'a> {
    /// Nothing: the live line is erased.
    Nothing,
    /// A result line for this status and text.
    Result(Status, &'a str),
    /// The progress bar itself, as a lasting line at its real value and without the
    /// head glint. A task has no such line and leaves nothing.
    Kept,
}

/// What a region is started with.
pub(crate) struct Spec {
    pub(crate) term: Term,
    pub(crate) stream: Stream,
    pub(crate) theme: Arc<dyn Theme + Send + Sync>,
    pub(crate) body: Body,
    pub(crate) motion: bool,
    pub(crate) frame_ms: u64,
}

/// The live line: its terminal, theme and state, shared with the redraw thread.
pub(crate) struct Region {
    term: Term,
    stream: Stream,
    theme: Arc<dyn Theme + Send + Sync>,
    tokens: Tokens,
    frame: Mutex<Frame>,
    start: Instant,
    stop: AtomicBool,
    /// How many prompts are shown over the region; redraws wait for zero.
    paused: AtomicUsize,
    motion: bool,
    frame_ms: u64,
    /// Cells of a progress bar, which sets the easing's smallest step.
    bar_width: usize,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Region {
    /// Starts a region. With motion it hides the cursor, draws the first frame and
    /// starts the redraw thread; without, it only reserves the slot. Fails if another
    /// region is running.
    pub(crate) fn start(spec: Spec) -> io::Result<Arc<Region>> {
        let tokens = spec.theme.tokens().into_owned();
        let bar_width = tokens
            .get_live()
            .resolve(LiveKind::Progress, &tokens)
            .bar_width;
        let region = Arc::new(Region {
            term: spec.term,
            stream: spec.stream,
            theme: spec.theme,
            tokens,
            frame: Mutex::new(Frame { body: spec.body }),
            start: Instant::now(),
            stop: AtomicBool::new(false),
            paused: AtomicUsize::new(0),
            motion: spec.motion,
            frame_ms: spec.frame_ms.max(1),
            bar_width,
            thread: Mutex::new(None),
        });
        {
            let mut active = lock(&ACTIVE);
            if active.is_some() {
                return Err(io::Error::other("another live component is running"));
            }
            *active = Some(Arc::clone(&region));
        }
        if region.motion {
            if let Err(e) = region.begin() {
                region.release();
                let _ = region.term.show_cursor();
                return Err(e);
            }
        }
        Ok(region)
    }

    /// Hides the cursor, draws the first frame and starts the redraw thread.
    fn begin(self: &Arc<Self>) -> io::Result<()> {
        self.term.hide_cursor()?;
        self.draw(&lock(&self.frame))?;
        let region = Arc::clone(self);
        let handle = thread::Builder::new()
            .name("paloma-live".into())
            .spawn(move || region.run())?;
        *lock(&self.thread) = Some(handle);
        Ok(())
    }

    /// The redraw thread: one tick every `frame_ms` until `stop`.
    fn run(&self) {
        let interval = Duration::from_millis(self.frame_ms);
        let mut next = Instant::now() + interval;
        while !self.stop.load(Ordering::SeqCst) {
            let now = Instant::now();
            if now < next {
                thread::park_timeout(next - now);
                continue;
            }
            next = now + interval;
            let mut frame = lock(&self.frame);
            // Checked under the lock: `pause` and `restore` set their flag first, then
            // take the lock, so nothing is drawn after they return.
            if self.stop.load(Ordering::SeqCst) || self.paused.load(Ordering::SeqCst) > 0 {
                continue;
            }
            match &mut frame.body {
                Body::Task { counter, .. } => *counter = counter.wrapping_add(1),
                Body::Progress {
                    value,
                    shown,
                    total,
                    ..
                } => *shown = progress::ease(*shown, *value, *total, self.bar_width),
            }
            let _ = self.draw(&frame);
        }
    }

    /// Draws a task `view` with the theme, for this region's stream.
    fn task_line(&self, view: &TaskView) -> io::Result<String> {
        let ctx = Context::new(&self.tokens, self.stream);
        let mut out = String::new();
        self.theme
            .format_task(&mut out, &ctx, view)
            .map_err(io::Error::other)?;
        Ok(out)
    }

    /// Draws a progress `view` with the theme, for this region's stream.
    fn progress_line(&self, view: &ProgressView) -> io::Result<String> {
        let ctx = Context::new(&self.tokens, self.stream);
        let mut out = String::new();
        self.theme
            .format_progress(&mut out, &ctx, view)
            .map_err(io::Error::other)?;
        Ok(out)
    }

    /// Replaces the line under the cursor with the running line. The caller holds the
    /// frame lock.
    fn draw(&self, frame: &Frame) -> io::Result<()> {
        let mut line = match &frame.body {
            Body::Task { label, counter } => self.task_line(&TaskView::new(
                label,
                TaskState::Running,
                self.start.elapsed(),
                *counter,
            ))?,
            Body::Progress {
                label,
                shown,
                total,
                ..
            } => self.progress_line(&ProgressView::new(
                label,
                ProgressState::Running,
                shown.floor() as u64,
                *total,
                self.start.elapsed(),
            ))?,
        };
        // A line wider than the terminal would wrap and could not be cleared again.
        if let Some((_, width)) = self.term.size_checked() {
            let width = (width as usize).saturating_sub(1).max(1);
            line = console::truncate_str(&line, width, "…").into_owned();
        }
        self.term.clear_line()?;
        self.term.write_str(&line)?;
        self.term.flush()
    }

    /// Time since the region started.
    pub(crate) fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Prints a nested line above the running line.
    pub(crate) fn step(&self, text: &str) -> io::Result<()> {
        let ctx = Context::new(&self.tokens, self.stream);
        let mut out = String::new();
        self.theme
            .format_task_step(&mut out, &ctx, &TaskStepView::new(text))
            .map_err(io::Error::other)?;
        let frame = lock(&self.frame);
        if self.motion {
            self.term.clear_line()?;
        }
        self.term.write_line(&out)?;
        crate::flow::record_on(&self.term, &out);
        if self.motion && self.paused.load(Ordering::SeqCst) == 0 {
            self.draw(&frame)?;
        }
        self.term.flush()
    }

    /// Changes the running label and redraws at once.
    pub(crate) fn label(&self, label: String) -> io::Result<()> {
        let mut frame = lock(&self.frame);
        match &mut frame.body {
            Body::Task { label: current, .. } | Body::Progress { label: current, .. } => {
                *current = label
            }
        }
        if self.motion && self.paused.load(Ordering::SeqCst) == 0 {
            self.draw(&frame)?;
        }
        Ok(())
    }

    /// Raises a progress bar's value to `value`, at most its total. A lower value is
    /// ignored. Only stores it: the redraw thread draws it, eased.
    pub(crate) fn set(&self, value: u64) {
        self.update(|current, total| value.min(total).max(current));
    }

    /// Adds `by` to a progress bar's value, at most its total.
    pub(crate) fn inc(&self, by: u64) {
        self.update(|current, total| current.saturating_add(by).min(total));
    }

    /// Stores the value `f(current, total)` returns and wakes the redraw thread.
    fn update(&self, f: impl FnOnce(u64, u64) -> u64) {
        if let Body::Progress { value, total, .. } = &mut lock(&self.frame).body {
            *value = f(*value, *total);
        }
        self.wake();
    }

    /// A progress bar's real value (0 for a task).
    pub(crate) fn value(&self) -> u64 {
        match lock(&self.frame).body {
            Body::Progress { value, .. } => value,
            Body::Task { .. } => 0,
        }
    }

    /// Stops the region. Leaves what `leave` says: a result line, or nothing (the
    /// running line is only erased). The cursor comes back and the slot is freed even
    /// if writing fails; returns the elapsed time.
    pub(crate) fn finish(&self, leave: Leave) -> io::Result<Duration> {
        self.stop.store(true, Ordering::SeqCst);
        let handle = lock(&self.thread).take();
        if let Some(handle) = handle {
            handle.thread().unpark();
            let _ = handle.join();
        }
        let elapsed = self.start.elapsed();
        let frame = lock(&self.frame);
        let mut done = Ok(());
        if self.motion {
            done = self.term.clear_line();
        }
        let line = match (leave, &frame.body) {
            (Leave::Nothing, _) | (Leave::Kept, Body::Task { .. }) => None,
            (Leave::Result(status, text), Body::Task { counter, .. }) => Some(self.task_line(
                &TaskView::new(text, TaskState::Finished(status), elapsed, *counter),
            )),
            (Leave::Result(status, text), Body::Progress { value, total, .. }) => {
                Some(self.progress_line(&ProgressView::new(
                    text,
                    ProgressState::Finished(status),
                    *value,
                    *total,
                    elapsed,
                )))
            }
            (
                Leave::Kept,
                Body::Progress {
                    label,
                    value,
                    total,
                    ..
                },
            ) => Some(self.progress_line(&ProgressView::new(
                label,
                ProgressState::Kept,
                *value,
                *total,
                elapsed,
            ))),
        };
        if let Some(line) = line {
            done = done.and(line).and_then(|line| {
                self.term.write_line(&line)?;
                crate::flow::record_on(&self.term, &line);
                Ok(())
            });
        }
        if self.motion {
            let _ = self.term.show_cursor();
        }
        let _ = self.term.flush();
        drop(frame);
        self.release();
        done.map(|_| elapsed)
    }

    /// Frees the slot, unless a newer region took it.
    fn release(&self) {
        let mut active = lock(&ACTIVE);
        if active
            .as_ref()
            .is_some_and(|r| std::ptr::eq(Arc::as_ptr(r), self))
        {
            *active = None;
        }
    }

    /// Wakes the redraw thread so it sees `stop` now.
    fn wake(&self) {
        if let Some(handle) = lock(&self.thread).as_ref() {
            handle.thread().unpark();
        }
    }

    /// A prompt is about to draw: erase the line and give the cursor back.
    fn pause(&self) {
        self.paused.fetch_add(1, Ordering::SeqCst);
        let _frame = lock(&self.frame);
        if self.motion && !self.stop.load(Ordering::SeqCst) {
            let _ = self.term.clear_line();
            let _ = self.term.show_cursor();
            let _ = self.term.flush();
        }
    }

    /// The prompt is done: hide the cursor again and draw the line where it left off.
    fn resume(&self) {
        let before = self.paused.fetch_sub(1, Ordering::SeqCst);
        if before == 1 && self.motion && !self.stop.load(Ordering::SeqCst) {
            let frame = lock(&self.frame);
            let _ = self.term.hide_cursor();
            let _ = self.draw(&frame);
        }
    }
}

/// Pauses the running region, if any, while it exists: a prompt draws where the
/// spinner was, and the spinner comes back below the prompt's last line.
pub(crate) struct Pause(Option<Arc<Region>>);

impl Pause {
    /// Pauses the active region, if there is one.
    pub(crate) fn new() -> Self {
        let region = lock(&ACTIVE).clone();
        if let Some(region) = &region {
            region.pause();
        }
        Pause(region)
    }
}

/// Tests that assert on a running region's output take this lock exclusively; every
/// prompt renderer created in a test holds it shared. A renderer pauses the app-wide
/// region, so without it a prompt test running in parallel would pause a task test's
/// spinner.
#[cfg(test)]
pub(crate) static TEST_REGION: std::sync::RwLock<()> = std::sync::RwLock::new(());

impl Drop for Pause {
    fn drop(&mut self) {
        if let Some(region) = self.0.take() {
            region.resume();
        }
    }
}

/// Stops any running live component, erases its line and shows the cursor again on
/// the terminals. The live part of [`terminal::restore`](crate::terminal::restore).
pub(crate) fn restore() {
    let region = lock(&ACTIVE).take();
    if let Some(region) = region {
        // Not joined: this may run on a signal-handling thread.
        region.stop.store(true, Ordering::SeqCst);
        region.wake();
        let _frame = lock(&region.frame);
        if region.motion {
            let _ = region.term.clear_line();
            let _ = region.term.show_cursor();
            let _ = region.term.flush();
        }
    }
    for term in [Term::stderr(), Term::stdout()] {
        if term.is_term() {
            let _ = term.show_cursor();
            let _ = term.flush();
        }
    }
}
