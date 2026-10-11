//! What paloma knows about the terminal it draws on.
//!
//! [`capabilities`] reads the stream and the environment once per call. Apps and themes
//! use it to choose a palette or a glyph set; paloma uses it to style output for the
//! right stream.
use std::{
    env, io,
    sync::atomic::{AtomicBool, Ordering},
};

use console::Term;

use crate::theme::CursorShape;

/// Stops any running live component, erases its line, shows the cursor again, gives
/// the cursor its terminal shape back if a prompt changed it, and leaves the raw mode
/// a prompt may hold the terminal in.
///
/// paloma installs no signal handler. Call this from the app's own Ctrl+C handler
/// before exiting, so the terminal is left as it was found. Safe to call from another
/// thread, and when nothing is running. Only terminals are written to.
///
/// ```rust,no_run
/// # fn handler() {
/// paloma::terminal::restore();
/// std::process::exit(130);
/// # }
/// ```
pub fn restore() {
    crate::live::engine::restore();
    crate::keys::restore();
    if SHAPED.swap(false, Ordering::SeqCst) {
        for term in [Term::stderr(), Term::stdout()] {
            if term.is_term() {
                let _ = term.write_str(RESET_CURSOR_SHAPE);
                let _ = term.flush();
            }
        }
    }
}

/// Whether a prompt changed the cursor shape and has not reset it yet.
static SHAPED: AtomicBool = AtomicBool::new(false);

/// DECSCUSR with 0: the terminal's own cursor shape.
const RESET_CURSOR_SHAPE: &str = "\x1b[0 q";

/// The DECSCUSR parameter for `shape`: blinking block 1, steady block 2, blinking
/// underline 3, steady underline 4, blinking bar 5, steady bar 6. `None` for
/// [`CursorShape::Terminal`], which leaves the cursor alone.
pub(crate) fn cursor_shape_code(shape: CursorShape, blink: bool) -> Option<u8> {
    let blinking = match shape {
        CursorShape::Block => 1,
        CursorShape::Underline => 3,
        CursorShape::Bar => 5,
        _ => return None,
    };
    Some(if blink { blinking } else { blinking + 1 })
}

/// Gives the cursor `shape` on `term` (a terminal). Returns whether anything was
/// written; [`reset_cursor_shape`] undoes it.
pub(crate) fn set_cursor_shape(term: &Term, shape: CursorShape, blink: bool) -> io::Result<bool> {
    let Some(code) = cursor_shape_code(shape, blink) else {
        return Ok(false);
    };
    if !escapes_supported(term) {
        return Ok(false);
    }
    SHAPED.store(true, Ordering::SeqCst);
    term.write_str(&format!("\x1b[{code} q"))?;
    Ok(true)
}

/// Gives the cursor its terminal shape back after [`set_cursor_shape`].
pub(crate) fn reset_cursor_shape(term: &Term) -> io::Result<()> {
    SHAPED.store(false, Ordering::SeqCst);
    term.write_str(RESET_CURSOR_SHAPE)?;
    term.flush()
}

/// Whether `term` understands escape sequences such as DECSCUSR. A Windows console
/// does once virtual terminal processing is on (paloma turns it on when it can); a
/// terminal elsewhere always does.
#[cfg(windows)]
fn escapes_supported(term: &Term) -> bool {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, SetConsoleMode, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
    };

    if term.features().is_msys_tty() {
        return true;
    }
    let handle = term.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE;
    let mut mode = 0;
    if unsafe { GetConsoleMode(handle, &mut mode) } == 0 {
        return false;
    }
    mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING != 0
        || unsafe { SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) } != 0
}

#[cfg(not(windows))]
fn escapes_supported(_term: &Term) -> bool {
    true
}

/// Where a component writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Stream {
    /// Standard error: the default for interface output (prompts, messages, progress).
    #[default]
    Stderr,
    /// Standard output: usually for results.
    Stdout,
}

impl Stream {
    /// The `console` terminal for this stream.
    pub fn term(self) -> Term {
        match self {
            Stream::Stderr => Term::stderr(),
            Stream::Stdout => Term::stdout(),
        }
    }
}

/// How many colours the terminal shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorDepth {
    /// The 16 basic colours.
    Basic,
    /// The 256-colour palette (`TERM` contains `256color`).
    Ansi256,
    /// 24-bit colour (`COLORTERM` is `truecolor` or `24bit`).
    TrueColor,
}

/// What a stream's terminal can show.
#[derive(Clone, Copy, Debug)]
pub struct Capabilities {
    is_tty: bool,
    colors: bool,
    unicode: bool,
    width: usize,
    depth: ColorDepth,
}

impl Capabilities {
    /// The stream is an interactive terminal (not a pipe or a file).
    pub fn is_tty(&self) -> bool {
        self.is_tty
    }

    /// Colours are shown (`console`'s decision: a terminal, no `NO_COLOR`, or
    /// `CLICOLOR_FORCE`).
    pub fn colors(&self) -> bool {
        self.colors
    }

    /// Unicode glyphs are expected to render (`false` for `TERM=linux` and `TERM=dumb`).
    pub fn unicode(&self) -> bool {
        self.unicode
    }

    /// The terminal width in columns (80 when unknown).
    pub fn width(&self) -> usize {
        self.width
    }

    /// The colour depth from `COLORTERM` and `TERM`.
    pub fn depth(&self) -> ColorDepth {
        self.depth
    }
}

/// Detects the capabilities of `stream`'s terminal.
pub fn capabilities(stream: Stream) -> Capabilities {
    let term = stream.term();
    let term_var = env::var("TERM").unwrap_or_default();
    let depth = if matches!(env::var("COLORTERM").as_deref(), Ok("truecolor" | "24bit")) {
        ColorDepth::TrueColor
    } else if term_var.contains("256color") {
        ColorDepth::Ansi256
    } else {
        ColorDepth::Basic
    };
    Capabilities {
        is_tty: term.is_term(),
        colors: match stream {
            Stream::Stderr => console::colors_enabled_stderr(),
            Stream::Stdout => console::colors_enabled(),
        },
        unicode: !matches!(term_var.as_str(), "linux" | "dumb"),
        width: term.size_checked().map_or(80, |(_, w)| w as usize),
        depth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decscusr_codes_for_each_shape_and_blink() {
        let codes: Vec<_> = [
            CursorShape::Terminal,
            CursorShape::Block,
            CursorShape::Underline,
            CursorShape::Bar,
        ]
        .into_iter()
        .map(|shape| {
            (
                cursor_shape_code(shape, true),
                cursor_shape_code(shape, false),
            )
        })
        .collect();
        assert_eq!(
            codes,
            [
                (None, None),
                (Some(1), Some(2)),
                (Some(3), Some(4)),
                (Some(5), Some(6)),
            ]
        );
    }
}
