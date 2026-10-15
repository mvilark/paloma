//! A progress bar: a label and a bar that fills toward a total, then a lasting line.
use std::{sync::Arc, time::Duration};

use console::Term;

use crate::{
    live::engine::{Body, Leave, Region, Spec},
    output::message::Status,
    terminal::Stream,
    theme::{app, LiveKind, Theme},
    Result,
};

/// Whether a progress bar is still filling, was left on screen, or ended with a status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProgressState {
    /// The bar is filling.
    Running,
    /// The bar was left on screen as a lasting line (see [`RunningProgress::keep`]).
    Kept,
    /// The bar was replaced by a result line with this status.
    Finished(Status),
}

/// What a theme gets to draw a progress line.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct ProgressView<'a> {
    /// The label; for [`Finished`](ProgressState::Finished), the result text.
    pub text: &'a str,
    /// Whether the bar is filling, kept or finished.
    pub state: ProgressState,
    /// The value drawn: the eased one while running, the real one when kept.
    pub value: u64,
    /// The value that means done.
    pub total: u64,
    /// Time since the bar started.
    pub elapsed: Duration,
}

impl<'a> ProgressView<'a> {
    /// A view of a progress line.
    pub fn new(
        text: &'a str,
        state: ProgressState,
        value: u64,
        total: u64,
        elapsed: Duration,
    ) -> Self {
        ProgressView {
            text,
            state,
            value,
            total,
            elapsed,
        }
    }
}

/// A progress bar for `label`, out of `total` units. Start it with
/// [`Progress::start`].
pub fn progress(label: impl Into<String>, total: u64) -> Progress {
    Progress::new(label, total)
}

/// A bar that fills as work advances, then stays on screen or becomes a result line.
///
/// It lives on the same live region as [`Task`](crate::Task), so it works with no
/// setup and follows the same rules: the app theme draws it, [`Options`](crate::Options)
/// decide the stream (stderr by default), the frame rate (40 ms) and whether it moves at
/// all. Motion is off when the stream is not a terminal or the `CI` environment variable
/// is set; then nothing is drawn until [`keep`](RunningProgress::keep) or a result
/// line, which print once.
///
/// While it runs the bar eases toward the value you set and never moves backwards, so
/// a burst of [`set`](RunningProgress::set) or [`inc`](RunningProgress::inc) calls costs
/// nothing. Only one live component runs at a time. A prompt shown while the bar runs
/// pauses it and brings it back after the answer. paloma installs no signal handler:
/// call [`terminal::restore`](crate::terminal::restore) from your Ctrl+C handler.
///
/// ```rust,no_run
/// # fn main() -> paloma::Result {
/// let bar = paloma::progress("Downloading...", 200).start()?;
/// for _ in 0..200 {
///     // ... fetch a chunk ...
///     bar.inc(1)?;
/// }
/// bar.success("Downloaded 200 chunks")?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Progress {
    label: String,
    total: u64,
    theme: Option<Arc<dyn Theme + Send + Sync>>,
    stream: Option<Stream>,
}

impl Progress {
    /// A progress bar for `label`, such as "Downloading...", out of `total` units. With
    /// a `total` of 0 the bar stays empty with `0%`, and [`keep`](RunningProgress::keep)
    /// shows it full (a job of nothing is done).
    pub fn new(label: impl Into<String>, total: u64) -> Self {
        Progress {
            label: label.into(),
            total,
            theme: None,
            stream: None,
        }
    }

    /// Draws this bar with `theme` instead of the app theme. Takes the theme by value
    /// (not `&t` like other components) because the bar redraws from another thread.
    pub fn theme(mut self, theme: impl Theme + Send + Sync + 'static) -> Self {
        self.theme = Some(Arc::new(theme));
        self
    }

    /// Where the bar draws. Left unset: the `progress` options, then `live`, then
    /// stderr.
    pub fn stream(mut self, stream: Stream) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Shows the bar at 0: with motion, a line redrawn every frame; without motion,
    /// nothing yet. Fails if another live component is running.
    pub fn start(self) -> Result<RunningProgress> {
        let stream = self.resolved_stream(&crate::options::current());
        self.launch(&stream.term(), stream)
    }

    /// Like [`start`](Self::start) but draws on `term` (motion is off unless `term` is a
    /// terminal).
    pub fn start_on(self, term: &Term) -> Result<RunningProgress> {
        let stream = self.resolved_stream(&crate::options::current());
        self.launch(term, stream)
    }

    /// The stream from the bar, its options group, the `live` family, then stderr.
    fn resolved_stream(&self, options: &crate::Options) -> Stream {
        self.stream
            .or(options.live_member(LiveKind::Progress).stream)
            .or(options.live.stream)
            .unwrap_or_default()
    }

    fn launch(self, term: &Term, stream: Stream) -> Result<RunningProgress> {
        let options = crate::options::current();
        let member = options.live_member(LiveKind::Progress);
        let motion = term.is_term()
            && member
                .motion
                .or(options.live.motion)
                .unwrap_or_else(|| std::env::var_os("CI").is_none());
        let frame_ms = member.frame_ms.or(options.live.frame_ms).unwrap_or(40);
        let region = Region::start(Spec {
            term: term.clone(),
            stream,
            theme: self.theme.unwrap_or_else(app::current),
            body: Body::progress(self.label, self.total),
            motion,
            frame_ms,
        })?;
        Ok(RunningProgress {
            region,
            finished: false,
        })
    }
}

/// A started progress bar. Dropping it without finishing erases the bar and restores
/// the cursor, printing nothing (like [`clear`](Self::clear)).
pub struct RunningProgress {
    region: Arc<Region>,
    finished: bool,
}

impl RunningProgress {
    /// Sets the value; it is clamped to the total. A value below the current one is
    /// ignored, so the bar never moves backwards.
    pub fn set(&self, value: u64) -> Result {
        self.region.set(value);
        Ok(())
    }

    /// Adds `by` to the value, clamped to the total.
    pub fn inc(&self, by: u64) -> Result {
        self.region.inc(by);
        Ok(())
    }

    /// Changes the label.
    pub fn label(&self, label: impl Into<String>) -> Result {
        Ok(self.region.label(label.into())?)
    }

    /// The current value (not the eased one on screen).
    pub fn value(&self) -> u64 {
        self.region.value()
    }

    /// Time since [`start`](Progress::start).
    pub fn elapsed(&self) -> Duration {
        self.region.elapsed()
    }

    /// Leaves the bar on screen as a lasting line, at the current value and with no
    /// glint; returns the elapsed time. With a total of 0 the bar is shown full.
    pub fn keep(mut self) -> Result<Duration> {
        self.finished = true;
        Ok(self.region.finish(Leave::Kept)?)
    }

    /// Replaces the bar with a result line for `status` with `text`, drawn like a
    /// task's; returns the elapsed time.
    pub fn finish(mut self, status: Status, text: impl Into<String>) -> Result<Duration> {
        self.finished = true;
        Ok(self.region.finish(Leave::Result(status, &text.into()))?)
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

    /// Removes the bar and leaves nothing.
    pub fn clear(mut self) -> Result<Duration> {
        self.finished = true;
        Ok(self.region.finish(Leave::Nothing)?)
    }
}

impl Drop for RunningProgress {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.region.finish(Leave::Nothing);
        }
    }
}

/// One easing step: how far the bar on screen (`shown`) moves toward `value` in one
/// frame. A quarter of the distance, but never less than a quarter of a cell
/// (`total / width / 4`) so the last stretch does not crawl. Never goes backwards and
/// never passes `value`; with a `total` of 0 there is nothing to ease.
pub(crate) fn ease(shown: f64, value: u64, total: u64, width: usize) -> f64 {
    let value = value as f64;
    if total == 0 || shown >= value {
        return shown;
    }
    let quarter_cell = total as f64 / width.max(1) as f64 / 4.0;
    (shown + ((value - shown) * 0.25).max(quarter_cell)).min(value)
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

    fn bar(term: &Term, total: u64) -> RunningProgress {
        Progress::new("Working", total)
            .theme(SimpleTheme)
            .start_on(term)
            .unwrap()
    }

    #[test]
    fn easing_reaches_the_value_and_never_goes_backwards() {
        let (total, width) = (100, 20);
        for value in [1, 7, 50, 100] {
            let mut shown = 0.0;
            let mut frames = 0;
            while shown < value as f64 {
                let next = ease(shown, value, total, width);
                assert!(next > shown, "stuck at {shown} for {value}");
                assert!(next <= value as f64);
                shown = next;
                frames += 1;
                assert!(frames < 1000, "never reaches {value}");
            }
            assert_eq!(shown, value as f64);
        }
        // At the value, or past it (a value never decreases, but be safe): unchanged.
        assert_eq!(ease(40.0, 40, total, width), 40.0);
        assert_eq!(ease(60.0, 40, total, width), 60.0);
        // A quarter of the distance, or a quarter of a cell when that is more.
        assert_eq!(ease(0.0, 100, 100, 20), 25.0);
        assert_eq!(ease(98.0, 100, 100, 20), 99.25);
        assert_eq!(ease(99.9, 100, 100, 20), 100.0);
        // Nothing to ease with no total, and a zero width does not divide by zero.
        assert_eq!(ease(0.0, 0, 0, 20), 0.0);
        assert!(ease(0.0, 10, 100, 0) > 0.0);
    }

    // One test function: the live region is a global slot shared by the whole binary
    // (and shared with the task test through `TEST_REGION`).
    #[test]
    fn bars_print_one_line_and_keep_one_slot() {
        // No prompt renderer may pause the app-wide region while this test runs.
        let _exclusive = engine::TEST_REGION
            .write()
            .unwrap_or_else(|e| e.into_inner());
        let (term, buf) = pair();

        // Without motion (not a terminal): nothing at start or while it advances.
        let running = bar(&term, 4);
        assert_eq!(buf.drain(), "");
        running.set(1).unwrap();
        running.inc(1).unwrap();
        running.label("Still working").unwrap();
        assert_eq!(buf.drain(), "");
        assert_eq!(running.value(), 2);
        // `keep` prints exactly one line, at the real value, without a head.
        running.keep().unwrap();
        assert_eq!(
            buf.drain(),
            format!("{}{} Still working  50%\n", "=".repeat(15), "-".repeat(15))
        );

        // The result is one line, drawn like a task's (a bar shows no elapsed time unless
        // its token asks for it).
        for (status, line) in [
            (Status::Success, "success: done\n"),
            (Status::Warning, "warning: done\n"),
            (Status::Error, "error: done\n"),
        ] {
            let running = bar(&term, 4);
            running.set(3).unwrap();
            running.finish(status, "done").unwrap();
            assert_eq!(buf.drain(), line);
        }
        bar(&term, 4).success("done").unwrap();
        assert_eq!(buf.drain(), "success: done\n");

        // `clear` prints nothing at all.
        let running = bar(&term, 4);
        running.set(4).unwrap();
        running.clear().unwrap();
        assert_eq!(buf.drain(), "");

        // A lower value is ignored, `inc` and `set` clamp to the total.
        let running = bar(&term, 10);
        running.set(6).unwrap();
        running.set(2).unwrap();
        assert_eq!(running.value(), 6);
        running.inc(100).unwrap();
        assert_eq!(running.value(), 10);
        running.set(3).unwrap();
        running.set(1000).unwrap();
        assert_eq!(running.value(), 10);
        running.keep().unwrap();
        assert_eq!(buf.drain(), format!("{} Working  100%\n", "=".repeat(30)));

        // A total of 0 is an empty track with 0%, and kept it is a finished job.
        let running = bar(&term, 0);
        running.inc(5).unwrap();
        assert_eq!(running.value(), 0);
        running.keep().unwrap();
        assert_eq!(buf.drain(), format!("{} Working  100%\n", "=".repeat(30)));

        // A second start while one runs fails; the slot is free again after a drop,
        // which prints nothing.
        let first = bar(&term, 3);
        let second = Progress::new("Two", 3).theme(SimpleTheme).start_on(&term);
        assert!(second.is_err());
        let task = crate::Task::new("Task").theme(SimpleTheme).start_on(&term);
        assert!(task.is_err());
        drop(first);
        assert_eq!(buf.drain(), "");
        bar(&term, 3).success("ok").unwrap();
        assert_eq!(buf.drain(), "success: ok\n");
        // A task blocks a bar the same way.
        let task = crate::Task::new("Task")
            .theme(SimpleTheme)
            .start_on(&term)
            .unwrap();
        assert!(Progress::new("Bar", 3)
            .theme(SimpleTheme)
            .start_on(&term)
            .is_err());
        drop(task);
        buf.drain();

        // `restore` frees the slot.
        let stuck = bar(&term, 3);
        engine::restore();
        let next = bar(&term, 3);
        drop(stuck);
        drop(next);
        engine::restore();
        buf.drain();

        // With motion: hidden cursor, frames from a thread, the bar eases up to the
        // value and never back, no frames while paused, the cursor back at the end.
        let (term, buf) = pair();
        let region = Region::start(Spec {
            term: term.clone(),
            stream: Stream::Stderr,
            theme: Arc::new(SimpleTheme),
            body: Body::progress("Working".into(), 100),
            motion: true,
            frame_ms: 5,
        })
        .unwrap();
        let running = RunningProgress {
            region,
            finished: false,
        };
        let empty = format!("\x1b[?25l\r\x1b[2K{} Working  0%", "-".repeat(30));
        assert!(buf.text().starts_with(&empty), "{:?}", buf.text());
        running.set(50).unwrap();
        // 50% is 15 cells of 30.
        wait_for(
            &buf,
            &format!("{}{} Working  50%", "=".repeat(15), "-".repeat(15)),
        );
        // Frames only ever grow the fill. (A frame is a clear-line and then the line in
        // two writes, so skip the one the thread may be in the middle of.)
        let mut widths = Vec::new();
        for frame in buf.text().split("\r\x1b[2K").skip(1) {
            if frame.contains("Working") {
                widths.push(frame.chars().take_while(|c| *c == '=').count());
            }
        }
        assert!(widths.windows(2).all(|w| w[0] <= w[1]), "{widths:?}");
        assert_eq!(widths.last(), Some(&15));
        {
            let _pause = engine::Pause::new();
            assert!(buf.text().ends_with("\r\x1b[2K\x1b[?25h"));
            let len = buf.text().len();
            thread::sleep(Duration::from_millis(60));
            assert_eq!(buf.text().len(), len);
        }
        // The bar comes back where it left off, with the cursor hidden again.
        assert!(buf.text().contains(&format!(
            "\x1b[?25l\r\x1b[2K{}{} Working  50%",
            "=".repeat(15),
            "-".repeat(15)
        )));
        // `keep` stops the thread, then draws the real value once.
        running.set(100).unwrap();
        running.keep().unwrap();
        let out = buf.text();
        assert!(
            out.ends_with(&format!(
                "\r\x1b[2K{} Working  100%\n\x1b[?25h",
                "=".repeat(30)
            )),
            "{out:?}"
        );
        let len = out.len();
        thread::sleep(Duration::from_millis(30));
        assert_eq!(buf.text().len(), len);

        // Dropping a running bar restores the cursor and leaves no line.
        let (term, buf) = pair();
        drop(bar_with_motion(&term));
        let out = buf.text();
        assert!(out.starts_with("\x1b[?25l"), "{out:?}");
        assert!(out.ends_with("\r\x1b[2K\x1b[?25h"), "{out:?}");
        assert!(!out.contains('\n'), "{out:?}");

        // The slot is free for the next one.
        bar(&term, 3).clear().unwrap();
    }

    /// A bar with motion on, over a terminal that is not one.
    fn bar_with_motion(term: &Term) -> RunningProgress {
        let region = Region::start(Spec {
            term: term.clone(),
            stream: Stream::Stderr,
            theme: Arc::new(SimpleTheme),
            body: Body::progress("Working".into(), 10),
            motion: true,
            frame_ms: 5,
        })
        .unwrap();
        RunningProgress {
            region,
            finished: false,
        }
    }
}
