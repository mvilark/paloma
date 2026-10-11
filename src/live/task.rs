//! A task: a label with a spinner while work runs, then a result line.
use std::{sync::Arc, time::Duration};

use console::Term;

use crate::{
    live::engine::{Region, Spec},
    output::message::Status,
    terminal::Stream,
    theme::{app, LiveKind, Theme},
    Result,
};

/// Whether a task is still working or has ended with a status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TaskState {
    /// The spinner is turning.
    Running,
    /// The task ended with this status.
    Finished(Status),
}

/// What a theme gets to draw a task line.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct TaskView<'a> {
    /// The running label, or the result text once finished.
    pub text: &'a str,
    /// Whether the task is running or finished.
    pub state: TaskState,
    /// Time since the task started.
    pub elapsed: Duration,
    /// The spinner frame counter; the hook takes it modulo the number of frames.
    pub frame: usize,
}

impl<'a> TaskView<'a> {
    /// A view of a task line.
    pub fn new(text: &'a str, state: TaskState, elapsed: Duration, frame: usize) -> Self {
        TaskView {
            text,
            state,
            elapsed,
            frame,
        }
    }
}

/// What a theme gets to draw a nested step line of a task.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct TaskStepView<'a> {
    /// The step's text.
    pub text: &'a str,
}

impl<'a> TaskStepView<'a> {
    /// A view of a step line.
    pub fn new(text: &'a str) -> Self {
        TaskStepView { text }
    }
}

/// A task label shown with a spinner while work runs; see [`Task::start`].
pub fn task(label: impl Into<String>) -> Task {
    Task::new(label)
}

/// A label with a spinner that turns while work runs, then a result line.
///
/// Works with no setup: the app theme draws it, and [`Options`](crate::Options) decide
/// the stream (stderr by default), the frame rate and whether it moves at all. Motion
/// is off when the stream is not a terminal or the `CI` environment variable is set;
/// then nothing is drawn until a step or the result line, each printed once.
///
/// Only one live component runs at a time. A prompt shown while a task runs pauses
/// the spinner and brings it back after the answer. paloma installs no signal handler:
/// call [`terminal::restore`](crate::terminal::restore) from your Ctrl+C handler.
///
/// ```rust,no_run
/// # fn main() -> paloma::Result {
/// let task = paloma::task("Installing dependencies...").start()?;
/// task.step("Resolved 214 packages")?;
/// task.success("Installed dependencies")?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Task {
    label: String,
    theme: Option<Arc<dyn Theme + Send + Sync>>,
    stream: Option<Stream>,
}

impl Task {
    /// A task with this running label, such as "Installing dependencies...".
    pub fn new(label: impl Into<String>) -> Self {
        Task {
            label: label.into(),
            theme: None,
            stream: None,
        }
    }

    /// Draws this task with `theme` instead of the app theme. Takes the theme by value
    /// (not `&t` like other components) because the spinner redraws from another thread.
    pub fn theme(mut self, theme: impl Theme + Send + Sync + 'static) -> Self {
        self.theme = Some(Arc::new(theme));
        self
    }

    /// Where the task draws. Left unset: the `task` options, then `live`, then stderr.
    pub fn stream(mut self, stream: Stream) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Starts the task: with motion, a spinner line redrawn every frame; without
    /// motion, nothing yet. Fails if another live component is running.
    pub fn start(self) -> Result<RunningTask> {
        let stream = self.resolved_stream(&crate::options::current());
        self.launch(&stream.term(), stream)
    }

    /// Like [`start`](Self::start) but draws on `term` (motion is off unless `term` is a
    /// terminal).
    pub fn start_on(self, term: &Term) -> Result<RunningTask> {
        let stream = self.resolved_stream(&crate::options::current());
        self.launch(term, stream)
    }

    /// The stream from the task, its options group, the `live` family, then stderr.
    fn resolved_stream(&self, options: &crate::Options) -> Stream {
        self.stream
            .or(options.live_member(LiveKind::Task).stream)
            .or(options.live.stream)
            .unwrap_or_default()
    }

    fn launch(self, term: &Term, stream: Stream) -> Result<RunningTask> {
        let options = crate::options::current();
        let member = options.live_member(LiveKind::Task);
        let motion = term.is_term()
            && member
                .motion
                .or(options.live.motion)
                .unwrap_or_else(|| std::env::var_os("CI").is_none());
        let frame_ms = member.frame_ms.or(options.live.frame_ms).unwrap_or(80);
        let region = Region::start(Spec {
            term: term.clone(),
            stream,
            theme: self.theme.unwrap_or_else(app::current),
            label: self.label,
            motion,
            frame_ms,
        })?;
        Ok(RunningTask {
            region,
            finished: false,
        })
    }
}

/// A started task. Dropping it without finishing clears the spinner line and restores
/// the cursor, printing nothing.
pub struct RunningTask {
    region: Arc<Region>,
    finished: bool,
}

impl RunningTask {
    /// Prints a nested line under the running task (the spinner stays below it).
    pub fn step(&self, text: impl Into<String>) -> Result {
        Ok(self.region.step(&text.into())?)
    }

    /// Changes the running label.
    pub fn label(&self, label: impl Into<String>) -> Result {
        Ok(self.region.label(label.into())?)
    }

    /// Time since [`start`](Task::start).
    pub fn elapsed(&self) -> Duration {
        self.region.elapsed()
    }

    /// Stops the spinner and prints the result line for `status` with `text`;
    /// returns the elapsed time.
    pub fn finish(mut self, status: Status, text: impl Into<String>) -> Result<Duration> {
        self.finished = true;
        Ok(self.region.finish(Some((status, &text.into())))?)
    }

    /// `finish(Status::Success, text)`.
    pub fn success(self, text: impl Into<String>) -> Result<Duration> {
        self.finish(Status::Success, text)
    }

    /// `finish(Status::Warning, text)`.
    pub fn warning(self, text: impl Into<String>) -> Result<Duration> {
        self.finish(Status::Warning, text)
    }

    /// `finish(Status::Error, text)`.
    pub fn fail(self, text: impl Into<String>) -> Result<Duration> {
        self.finish(Status::Error, text)
    }

    /// Stops the spinner and erases its line, printing no result; step lines stay.
    pub fn clear(mut self) -> Result<Duration> {
        self.finished = true;
        Ok(self.region.finish(None)?)
    }
}

impl Drop for RunningTask {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.region.finish(None);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::{
        io::{self, Read, Write},
        os::fd::{AsRawFd, RawFd},
        sync::Mutex,
        thread,
        time::Instant,
    };

    use super::*;
    use crate::{live::engine, theme::SimpleTheme};

    /// A terminal backend that collects what is written. Not a tty (fd -1).
    #[derive(Clone, Debug, Default)]
    struct Buf(Arc<Mutex<Vec<u8>>>);

    impl Buf {
        fn drain(&self) -> String {
            let mut bytes = self.0.lock().unwrap();
            String::from_utf8(std::mem::take(&mut *bytes)).unwrap()
        }

        fn text(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl Read for Buf {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Ok(0)
        }
    }

    impl Write for Buf {
        fn write(&mut self, data: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(data);
            Ok(data.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl AsRawFd for Buf {
        fn as_raw_fd(&self) -> RawFd {
            -1
        }
    }

    fn pair() -> (Term, Buf) {
        let buf = Buf::default();
        (Term::read_write_pair(buf.clone(), buf.clone()), buf)
    }

    /// Waits until the buffer contains `needle`.
    fn wait_for(buf: &Buf, needle: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !buf.text().contains(needle) {
            assert!(
                Instant::now() < deadline,
                "no {needle:?} in {:?}",
                buf.text()
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    // One test function: the live region is a global slot shared by the whole binary.
    #[test]
    fn tasks_print_steps_and_results_and_keep_one_slot() {
        // No prompt renderer may pause the app-wide region while this test runs.
        let _exclusive = engine::TEST_REGION
            .write()
            .unwrap_or_else(|e| e.into_inner());
        // Without motion (not a terminal): nothing at start, then lines once each.
        let (term, buf) = pair();
        let task = Task::new("Working")
            .theme(SimpleTheme)
            .start_on(&term)
            .unwrap();
        assert_eq!(buf.drain(), "");
        task.step("a").unwrap();
        task.step("b").unwrap();
        task.label("Still working").unwrap();
        task.success("done").unwrap();
        assert_eq!(buf.drain(), "    - a\n    - b\nsuccess: done  0.0s\n");

        // Each status draws like a message of that status.
        for (status, line) in [
            (Status::Warning, "warning: careful  0.0s\n"),
            (Status::Error, "error: nope  0.0s\n"),
        ] {
            let task = Task::new("Working")
                .theme(SimpleTheme)
                .start_on(&term)
                .unwrap();
            task.finish(
                status,
                if status == Status::Error {
                    "nope"
                } else {
                    "careful"
                },
            )
            .unwrap();
            assert_eq!(buf.drain(), line);
        }

        // `clear` prints no result line; steps stay.
        let task = Task::new("Working")
            .theme(SimpleTheme)
            .start_on(&term)
            .unwrap();
        task.step("a").unwrap();
        task.clear().unwrap();
        assert_eq!(buf.drain(), "    - a\n");

        // A second start while one runs fails; the slot is free again after a drop.
        let first = Task::new("One").theme(SimpleTheme).start_on(&term).unwrap();
        let second = Task::new("Two").theme(SimpleTheme).start_on(&term);
        assert!(second.is_err());
        drop(first);
        assert_eq!(buf.drain(), "");
        let again = Task::new("Three")
            .theme(SimpleTheme)
            .start_on(&term)
            .unwrap();
        again.success("ok").unwrap();
        assert_eq!(buf.drain(), "success: ok  0.0s\n");

        // `restore` frees the slot and is safe when nothing runs.
        let stuck = Task::new("Stuck")
            .theme(SimpleTheme)
            .start_on(&term)
            .unwrap();
        engine::restore();
        let next = Task::new("Next")
            .theme(SimpleTheme)
            .start_on(&term)
            .unwrap();
        drop(stuck);
        drop(next);
        engine::restore();
        buf.drain();

        // With motion: hidden cursor, frames from a thread, steps above the line, no
        // frames while paused, the cursor back at the end.
        let (term, buf) = pair();
        let region = Region::start(Spec {
            term: term.clone(),
            stream: Stream::Stderr,
            theme: Arc::new(SimpleTheme),
            label: "Working".into(),
            motion: true,
            frame_ms: 5,
        })
        .unwrap();
        let running = RunningTask {
            region,
            finished: false,
        };
        assert!(buf.text().starts_with("\x1b[?25l\r\x1b[2K- Working  0."));
        wait_for(&buf, "\\ Working");
        running.step("a").unwrap();
        assert!(buf.text().contains("\r\x1b[2K    - a\n\r\x1b[2K"));
        {
            let _pause = engine::Pause::new();
            assert!(buf.text().ends_with("\r\x1b[2K\x1b[?25h"));
            let len = buf.text().len();
            thread::sleep(Duration::from_millis(60));
            assert_eq!(buf.text().len(), len);
        }
        assert!(buf.text().contains("\x1b[?25l\r\x1b[2K"));
        running.success("done").unwrap();
        let out = buf.text();
        assert!(out.contains("\r\x1b[2Ksuccess: done  0."), "{out:?}");
        assert!(out.ends_with("\n\x1b[?25h"), "{out:?}");
        let len = out.len();
        thread::sleep(Duration::from_millis(30));
        assert_eq!(buf.text().len(), len);

        // The slot is free for the next task.
        Task::new("After")
            .theme(SimpleTheme)
            .start_on(&term)
            .unwrap()
            .clear()
            .unwrap();
    }
}
