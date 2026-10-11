//! The live region: one line that is redrawn from a background thread while work runs.
//!
//! At most one region exists at a time (several at once is a later phase). It owns the
//! hidden cursor: [`Region::finish`], [`restore`] and the drop of a
//! [`RunningTask`](crate::RunningTask) all show the cursor again. Every write to the
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
    live::task::{TaskState, TaskStepView, TaskView},
    output::message::Status,
    terminal::Stream,
    theme::{Context, Theme, Tokens},
};

/// The region on screen, if any.
static ACTIVE: Mutex<Option<Arc<Region>>> = Mutex::new(None);

/// Locks `mutex`, ignoring poison: a panic in a theme hook must not stop the cursor
/// from coming back.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// What the redraw thread draws.
struct Frame {
    /// The running label.
    label: String,
    /// Index of the spinner frame on screen.
    counter: usize,
}

/// What a region is started with.
pub(crate) struct Spec {
    pub(crate) term: Term,
    pub(crate) stream: Stream,
    pub(crate) theme: Arc<dyn Theme + Send + Sync>,
    pub(crate) label: String,
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
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Region {
    /// Starts a region. With motion it hides the cursor, draws the first frame and
    /// starts the redraw thread; without, it only reserves the slot. Fails if another
    /// region is running.
    pub(crate) fn start(spec: Spec) -> io::Result<Arc<Region>> {
        let tokens = spec.theme.tokens().into_owned();
        let region = Arc::new(Region {
            term: spec.term,
            stream: spec.stream,
            theme: spec.theme,
            tokens,
            frame: Mutex::new(Frame {
                label: spec.label,
                counter: 0,
            }),
            start: Instant::now(),
            stop: AtomicBool::new(false),
            paused: AtomicUsize::new(0),
            motion: spec.motion,
            frame_ms: spec.frame_ms.max(1),
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
            frame.counter = frame.counter.wrapping_add(1);
            let _ = self.draw(&frame);
        }
    }

    /// Draws `view` with the theme, for this region's stream.
    fn line(&self, view: &TaskView) -> io::Result<String> {
        let ctx = Context::new(&self.tokens, self.stream);
        let mut out = String::new();
        self.theme
            .format_task(&mut out, &ctx, view)
            .map_err(io::Error::other)?;
        Ok(out)
    }

    /// Replaces the line under the cursor with the running line. The caller holds the
    /// frame lock.
    fn draw(&self, frame: &Frame) -> io::Result<()> {
        let view = TaskView::new(
            &frame.label,
            TaskState::Running,
            self.start.elapsed(),
            frame.counter,
        );
        let mut line = self.line(&view)?;
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
        if self.motion && self.paused.load(Ordering::SeqCst) == 0 {
            self.draw(&frame)?;
        }
        self.term.flush()
    }

    /// Changes the running label and redraws at once.
    pub(crate) fn label(&self, label: String) -> io::Result<()> {
        let mut frame = lock(&self.frame);
        frame.label = label;
        if self.motion && self.paused.load(Ordering::SeqCst) == 0 {
            self.draw(&frame)?;
        }
        Ok(())
    }

    /// Stops the region. Prints the result line for `result`, or only erases the
    /// running line for `None`. The cursor comes back and the slot is freed even if
    /// writing fails; returns the elapsed time.
    pub(crate) fn finish(&self, result: Option<(Status, &str)>) -> io::Result<Duration> {
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
        if let Some((status, text)) = result {
            let view = TaskView::new(text, TaskState::Finished(status), elapsed, frame.counter);
            done = done
                .and_then(|_| self.line(&view))
                .and_then(|line| self.term.write_line(&line));
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
