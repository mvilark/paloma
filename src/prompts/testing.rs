//! A terminal for unit tests: it records what is written and has nothing to read, so
//! presets and unattended answers can be checked without a pseudo-terminal.
use std::{
    fs::File,
    io::{self, Write},
    os::fd::{AsRawFd, RawFd},
    sync::{Arc, Mutex},
};

use console::Term;

/// What was written to a [`capture`] terminal.
#[derive(Clone, Default)]
pub(crate) struct Captured(Arc<Mutex<Vec<u8>>>);

impl Captured {
    /// The text written so far.
    pub(crate) fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

#[derive(Debug)]
struct Sink {
    out: Arc<Mutex<Vec<u8>>>,
    null: File,
}

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.out.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsRawFd for Sink {
    fn as_raw_fd(&self) -> RawFd {
        self.null.as_raw_fd()
    }
}

/// A terminal that is not a terminal (`is_term()` is false) and the text written to
/// it. Colours are switched off so the text is plain.
pub(crate) fn capture() -> (Term, Captured) {
    console::set_colors_enabled(false);
    console::set_colors_enabled_stderr(false);
    let captured = Captured::default();
    let sink = Sink {
        out: captured.0.clone(),
        null: File::create("/dev/null").unwrap(),
    };
    let term = Term::read_write_pair(File::open("/dev/null").unwrap(), sink);
    (term, captured)
}
