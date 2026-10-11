//! What paloma knows about the terminal it draws on.
//!
//! [`capabilities`] reads the stream and the environment once per call. Apps and themes
//! use it to choose a palette or a glyph set; paloma uses it to style output for the
//! right stream.
use std::env;

use console::Term;

pub use crate::live::engine::restore;

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
