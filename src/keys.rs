//! Reading keys with a timeout, for background checks (`change_delay`).
//!
//! `console` only offers a blocking key read, and switches the terminal to raw mode
//! only inside that read. A prompt with a background check needs to wake up while no
//! key comes (to start a check, or to show its result), so it holds a [`RawMode`]
//! guard for its whole life and reads with [`read_key_timeout`]: in canonical mode the
//! terminal would hand over input a line at a time, and a wait for one key would never
//! wake up.
//!
//! Only Unix (`libc`) and Windows (`windows-sys`) are supported. Elsewhere
//! [`TIMED_INPUT`] is false, [`read_key_timeout`] does not exist and background checks
//! never run.
use std::io;
#[cfg(any(unix, windows))]
use std::sync::{Mutex, MutexGuard};
#[cfg(any(unix, windows))]
use std::time::{Duration, Instant};

#[cfg(any(unix, windows))]
use console::{Key, Term};

/// Whether paloma can read keys with a timeout on this target.
pub(crate) const TIMED_INPUT: bool = cfg!(any(unix, windows));

/// Locks `mutex`, ignoring poison: restoring the terminal must work after a panic.
#[cfg(any(unix, windows))]
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// The milliseconds to wait for `remaining`, rounded up so a wait never ends early
/// and spins, and clamped to `max`.
#[cfg(any(unix, windows))]
fn millis(remaining: Duration, max: u64) -> u64 {
    let ms = remaining.as_micros().div_ceil(1000);
    u64::try_from(ms).unwrap_or(u64::MAX).min(max)
}

/// The terminal mode a prompt found, to put back when it ends (also from
/// [`restore`] on an app's signal-handling path).
#[cfg(unix)]
static SAVED: Mutex<Option<libc::termios>> = Mutex::new(None);

/// The console input mode a prompt found.
#[cfg(windows)]
static SAVED: Mutex<Option<u32>> = Mutex::new(None);

/// Keeps the terminal in raw mode while it lives, so keys arrive one at a time
/// without echo, and puts the mode it found back when dropped.
///
/// - Unix: the input `console` reads from (stdin if it is a terminal, otherwise
///   `/dev/tty`) is switched with `cfmakeraw`, keeping the output flags as `console`'s
///   own key read does, so `\n` still moves to the start of the next line. `^C` is
///   read as a byte, as `console` does inside its read.
/// - Windows: `console` already reads console events without line buffering, but it
///   only turns off `ENABLE_PROCESSED_INPUT` during its read. While a prompt waits
///   between keys, `Ctrl+C` would then raise a console control event instead of
///   arriving as a key, so the guard turns it off for the whole prompt.
#[cfg(any(unix, windows))]
pub(crate) struct RawMode {
    #[cfg(unix)]
    original: libc::termios,
    #[cfg(windows)]
    original: u32,
    /// The outermost guard: it set [`SAVED`] and clears it again.
    owner: bool,
}

#[cfg(any(unix, windows))]
impl RawMode {
    /// Switches the terminal to raw mode until the guard is dropped.
    pub(crate) fn enter() -> io::Result<RawMode> {
        let original = sys::get_mode()?;
        sys::set_mode(&sys::raw(&original))?;
        let mut saved = lock(&SAVED);
        let owner = saved.is_none();
        if owner {
            *saved = Some(original);
        }
        Ok(RawMode { original, owner })
    }
}

#[cfg(any(unix, windows))]
impl Drop for RawMode {
    fn drop(&mut self) {
        let mut saved = lock(&SAVED);
        // `restore` may have put the mode back already.
        if !self.owner || saved.take().is_some() {
            let _ = sys::set_mode(&self.original);
        }
    }
}

/// Puts back the terminal mode a prompt found, if one is in raw mode. For
/// [`terminal::restore`](crate::terminal::restore).
pub(crate) fn restore() {
    #[cfg(any(unix, windows))]
    if let Some(original) = lock(&SAVED).take() {
        let _ = sys::set_mode(&original);
    }
}

/// Waits up to `timeout` for a key and reads it with `term`, or returns `None` when no
/// key came. Call it while a [`RawMode`] guard lives: `console` saves and restores the
/// (raw) mode around its own read, so the terminal stays raw between keys.
#[cfg(any(unix, windows))]
pub(crate) fn read_key_timeout(term: &Term, timeout: Duration) -> io::Result<Option<Key>> {
    if sys::wait_for_key(Instant::now() + timeout)? {
        Ok(Some(term.read_key_raw()?))
    } else {
        Ok(None)
    }
}

#[cfg(unix)]
mod sys {
    use std::{
        fs::File,
        io,
        os::fd::{AsRawFd, RawFd},
        time::Instant,
    };

    use super::millis;

    /// The input `console` reads keys from: stdin when it is a terminal, otherwise
    /// `/dev/tty`.
    enum Input {
        Stdin,
        Tty(File),
    }

    impl Input {
        fn open() -> io::Result<Input> {
            if unsafe { libc::isatty(libc::STDIN_FILENO) } == 1 {
                Ok(Input::Stdin)
            } else {
                let tty = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open("/dev/tty")?;
                Ok(Input::Tty(tty))
            }
        }

        fn fd(&self) -> RawFd {
            match self {
                Input::Stdin => libc::STDIN_FILENO,
                Input::Tty(tty) => tty.as_raw_fd(),
            }
        }
    }

    fn check(result: libc::c_int) -> io::Result<()> {
        if result == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub(super) fn get_mode() -> io::Result<libc::termios> {
        let input = Input::open()?;
        let mut termios = std::mem::MaybeUninit::uninit();
        check(unsafe { libc::tcgetattr(input.fd(), termios.as_mut_ptr()) })?;
        Ok(unsafe { termios.assume_init() })
    }

    /// Applies `termios` once pending output is written (`TCSADRAIN`, as `console`
    /// does), without dropping typed keys.
    pub(super) fn set_mode(termios: &libc::termios) -> io::Result<()> {
        let input = Input::open()?;
        check(unsafe { libc::tcsetattr(input.fd(), libc::TCSADRAIN, termios) })
    }

    /// `original` in raw mode, with its output flags kept.
    pub(super) fn raw(original: &libc::termios) -> libc::termios {
        let mut termios = *original;
        make_raw(&mut termios);
        termios.c_oflag = original.c_oflag;
        termios
    }

    /// QNX has no `cfmakeraw`; this is what `console` does there.
    #[cfg(target_os = "nto")]
    fn make_raw(termios: &mut libc::termios) {
        termios.c_iflag &= !(libc::IGNBRK
            | libc::BRKINT
            | libc::PARMRK
            | libc::ISTRIP
            | libc::INLCR
            | libc::IGNCR
            | libc::ICRNL
            | libc::IXON);
        termios.c_oflag &= !libc::OPOST;
        termios.c_lflag &= !(libc::ECHO | libc::ECHONL | libc::ICANON | libc::ISIG | libc::IEXTEN);
        termios.c_cflag &= !(libc::CSIZE | libc::PARENB);
        termios.c_cflag |= libc::CS8;
    }

    #[cfg(not(target_os = "nto"))]
    fn make_raw(termios: &mut libc::termios) {
        unsafe { libc::cfmakeraw(termios) };
    }

    /// Waits until the input can be read (a key, or the end of the input, which the
    /// read then reports) or `deadline` passes. Retries with the remaining time when a
    /// signal interrupts the wait.
    pub(super) fn wait_for_key(deadline: Instant) -> io::Result<bool> {
        let input = Input::open()?;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let ms = millis(remaining, i32::MAX as u64) as libc::c_int;
            match wait(input.fd(), ms) {
                Err(e) if e.kind() == io::ErrorKind::Interrupted && !remaining.is_zero() => {}
                Err(e) if e.kind() == io::ErrorKind::Interrupted => return Ok(false),
                other => return other,
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn wait(fd: RawFd, ms: libc::c_int) -> io::Result<bool> {
        poll(fd, ms)
    }

    /// macOS cannot `poll` a terminal; `select` works (as in `console`).
    #[cfg(target_os = "macos")]
    fn wait(fd: RawFd, ms: libc::c_int) -> io::Result<bool> {
        if unsafe { libc::isatty(fd) } != 1 {
            return poll(fd, ms);
        }
        unsafe {
            let mut read: libc::fd_set = std::mem::zeroed();
            libc::FD_ZERO(&mut read);
            libc::FD_SET(fd, &mut read);
            let mut timeout = libc::timeval {
                tv_sec: (ms / 1000) as _,
                tv_usec: ((ms % 1000) * 1000) as _,
            };
            let ret = libc::select(
                fd + 1,
                &mut read,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut timeout,
            );
            if ret < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(libc::FD_ISSET(fd, &read))
            }
        }
    }

    /// Any event counts: a hang-up or an error is reported by the read that follows.
    fn poll(fd: RawFd, ms: libc::c_int) -> io::Result<bool> {
        let mut pollfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ret = unsafe { libc::poll(&mut pollfd, 1, ms) };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(ret > 0 && pollfd.revents != 0)
        }
    }
}

#[cfg(windows)]
mod sys {
    use std::{io, mem, time::Instant};

    use windows_sys::Win32::{
        Foundation::{HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::{
            Console::{
                GetConsoleMode, GetStdHandle, PeekConsoleInputW, ReadConsoleInputW, SetConsoleMode,
                ENABLE_PROCESSED_INPUT, INPUT_RECORD, KEY_EVENT, STD_INPUT_HANDLE,
            },
            Threading::WaitForSingleObject,
        },
    };

    use super::millis;

    /// The console input handle `console` reads keys from.
    fn input() -> io::Result<HANDLE> {
        let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(handle)
        }
    }

    pub(super) fn get_mode() -> io::Result<u32> {
        let mut mode = 0;
        if unsafe { GetConsoleMode(input()?, &mut mode) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(mode)
    }

    pub(super) fn set_mode(mode: &u32) -> io::Result<()> {
        if unsafe { SetConsoleMode(input()?, *mode) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// `original` without `ENABLE_PROCESSED_INPUT`, so `Ctrl+C` is a key event.
    pub(super) fn raw(original: &u32) -> u32 {
        original & !ENABLE_PROCESSED_INPUT
    }

    /// Waits until a key press is the next console input event, or `deadline`
    /// passes. Other events (key releases, mouse, focus, resize) are consumed, as
    /// `console`'s read would skip them.
    pub(super) fn wait_for_key(deadline: Instant) -> io::Result<bool> {
        let handle = input()?;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            // `INFINITE` is `u32::MAX`; never wait that long.
            let ms = millis(remaining, u64::from(u32::MAX - 1)) as u32;
            match unsafe { WaitForSingleObject(handle, ms) } {
                WAIT_OBJECT_0 => {}
                WAIT_TIMEOUT => return Ok(false),
                _ => return Err(io::Error::last_os_error()),
            }
            let mut record: INPUT_RECORD = unsafe { mem::zeroed() };
            let mut count = 0;
            if unsafe { PeekConsoleInputW(handle, &mut record, 1, &mut count) } == 0 {
                return Err(io::Error::last_os_error());
            }
            if count == 1 {
                if record.EventType == KEY_EVENT as u16
                    && unsafe { record.Event.KeyEvent.bKeyDown } != 0
                {
                    return Ok(true);
                }
                if unsafe { ReadConsoleInputW(handle, &mut record, 1, &mut count) } == 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            if remaining.is_zero() {
                return Ok(false);
            }
        }
    }
}

#[cfg(all(test, any(unix, windows)))]
mod tests {
    use super::*;

    #[test]
    fn waits_round_up_to_whole_milliseconds() {
        assert_eq!(millis(Duration::ZERO, 100), 0);
        assert_eq!(millis(Duration::from_micros(1), 100), 1);
        assert_eq!(millis(Duration::from_micros(1500), 100), 2);
        assert_eq!(millis(Duration::from_secs(5), 100), 100);
    }
}
