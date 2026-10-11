//! Inline text helpers: a hyperlink and a colour gradient. Both are `Display`, so they
//! go inside any `format!`, message or block.
use std::fmt;

use console::{Color, Style};

use crate::{
    terminal::{self, ColorDepth, Stream},
    theme::{app, Role},
};

/// A hyperlink; see [`link`].
#[derive(Clone, Debug)]
pub struct Link {
    url: String,
    text: Option<String>,
    stream: Stream,
}

/// A hyperlink to `url`, showing the URL itself.
///
/// On an interactive terminal that shows colours it is an OSC 8 hyperlink, drawn in
/// the app theme's `accent` role and underlined; anywhere else it is plain text.
///
/// ```rust
/// println!("Docs: {}", paloma::text::link("https://example.com").stream(paloma::terminal::Stream::Stdout));
/// ```
pub fn link(url: impl Into<String>) -> Link {
    Link {
        url: url.into(),
        text: None,
        stream: Stream::Stderr,
    }
}

impl Link {
    /// Shows `text` instead of the URL. Where hyperlinks are not shown, the URL is
    /// printed after it in parentheses.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// The stream the link is printed to, so styling follows that stream's terminal.
    /// Default stderr.
    pub fn stream(mut self, stream: Stream) -> Self {
        self.stream = stream;
        self
    }
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let caps = terminal::capabilities(self.stream);
        let shown = self.text.as_deref().unwrap_or(&self.url);
        if !(caps.is_tty() && caps.colors()) {
            return match &self.text {
                Some(text) => write!(f, "{text} ({})", self.url),
                None => write!(f, "{}", self.url),
            };
        }
        let tokens = app::current().tokens().into_owned();
        let style = tokens.get_palette().get(Role::accent).clone().underlined();
        let styled = match self.stream {
            Stream::Stderr => style.for_stderr().apply_to(shown),
            Stream::Stdout => style.for_stdout().apply_to(shown),
        };
        write!(f, "\x1b]8;;{}\x1b\\{styled}\x1b]8;;\x1b\\", self.url)
    }
}

/// A colour gradient across text; see [`gradient`].
#[derive(Clone, Debug)]
pub struct Gradient {
    text: String,
    from: (u8, u8, u8),
    to: (u8, u8, u8),
    style: Style,
    fallback: Option<Style>,
    stream: Stream,
}

/// `text` with each character coloured on a line from `from` to `to` (RGB), when the
/// terminal shows 24-bit colour. Elsewhere every character gets the fallback style
/// (the app theme's `accent_strong` role unless set with [`Gradient::fallback`]).
///
/// ```rust
/// eprintln!("{}", paloma::text::gradient("Ultraviolet", (213, 198, 255), (109, 74, 255)));
/// ```
pub fn gradient(text: impl Into<String>, from: (u8, u8, u8), to: (u8, u8, u8)) -> Gradient {
    Gradient {
        text: text.into(),
        from,
        to,
        style: Style::new(),
        fallback: None,
        stream: Stream::Stderr,
    }
}

impl Gradient {
    /// Attributes for every character, such as `Style::new().bold()`. The gradient sets
    /// the foreground colour on top.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// The style used where 24-bit colour is not available.
    pub fn fallback(mut self, style: Style) -> Self {
        self.fallback = Some(style);
        self
    }

    /// The stream the text is printed to. Default stderr.
    pub fn stream(mut self, stream: Stream) -> Self {
        self.stream = stream;
        self
    }
}

impl fmt::Display for Gradient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let caps = terminal::capabilities(self.stream);
        let for_stream = |style: Style| match self.stream {
            Stream::Stderr => style.for_stderr(),
            Stream::Stdout => style.for_stdout(),
        };
        if caps.depth() != ColorDepth::TrueColor {
            let style = self.fallback.clone().unwrap_or_else(|| {
                let tokens = app::current().tokens().into_owned();
                tokens.get_palette().get(Role::accent_strong).clone()
            });
            for c in self.text.chars() {
                write!(f, "{}", for_stream(style.clone()).apply_to(c))?;
            }
            return Ok(());
        }
        let n = self.text.chars().count().max(2) - 1;
        for (i, c) in self.text.chars().enumerate() {
            let lerp =
                |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * i as f32 / n as f32) as u8;
            let color = Color::TrueColor(
                lerp(self.from.0, self.to.0),
                lerp(self.from.1, self.to.1),
                lerp(self.from.2, self.to.2),
            );
            write!(
                f,
                "{}",
                for_stream(self.style.clone().fg(color)).apply_to(c)
            )?;
        }
        Ok(())
    }
}
