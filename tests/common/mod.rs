//! Golden-file helper shared by the integration tests.
//!
//! `assert_snapshot("name", actual)` compares `actual` with `tests/snapshots/name.txt`.
//! Run with `UPDATE_SNAPSHOTS=1` to write the files instead, then review the diff.
// Each test binary uses some of these helpers.
#![allow(dead_code)]

use std::{fs, path::PathBuf};

/// Escape characters are shown as `␛` so snapshots stay readable in diffs.
pub fn visible(text: &str) -> String {
    text.replace('\x1b', "␛")
}

pub fn assert_snapshot(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(format!("{name}.txt"));
    let actual = visible(actual);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing snapshot {}; run with UPDATE_SNAPSHOTS=1",
            path.display()
        )
    });
    if expected != actual {
        let line = expected
            .lines()
            .zip(actual.lines())
            .position(|(e, a)| e != a)
            .unwrap_or_else(|| expected.lines().count().min(actual.lines().count()));
        panic!(
            "snapshot {name} differs from line {}:\n  expected: {:?}\n  actual:   {:?}\n\
             Run with UPDATE_SNAPSHOTS=1 to accept the new output.",
            line + 1,
            expected.lines().nth(line).unwrap_or("<end>"),
            actual.lines().nth(line).unwrap_or("<end>"),
        );
    }
}

/// A terminal that is not a terminal and the text written to it, for checking what
/// presets and unattended answers draw.
#[cfg(unix)]
pub mod capture {
    use std::{
        fs::File,
        io::{self, Write},
        os::fd::{AsRawFd, RawFd},
        sync::{Arc, Mutex},
    };

    use paloma::console::Term;

    #[derive(Clone, Default)]
    pub struct Captured(Arc<Mutex<Vec<u8>>>);

    impl Captured {
        /// Takes the text written so far.
        pub fn take(&self) -> String {
            let bytes = std::mem::take(&mut *self.0.lock().unwrap());
            String::from_utf8_lossy(&bytes).into_owned()
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

    pub fn term() -> (Term, Captured) {
        let captured = Captured::default();
        let sink = Sink {
            out: captured.0.clone(),
            null: File::create("/dev/null").unwrap(),
        };
        (
            Term::read_write_pair(File::open("/dev/null").unwrap(), sink),
            captured,
        )
    }
}
