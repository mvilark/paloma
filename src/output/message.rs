//! Status messages: description, hint, info, success, warning and error.
use console::Term;

use crate::{
    terminal::Stream,
    theme::{app, Context, Theme},
    Result,
};

/// The kind of a [`Message`]. Shared by every component that shows a status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Status {
    /// Neutral text that explains something.
    Description,
    /// A suggestion: what to try or type.
    Hint,
    /// Neutral information.
    Info,
    /// Something finished well.
    Success,
    /// Something needs attention.
    Warning,
    /// Something failed.
    Error,
}

/// What a theme gets to draw one message line.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct MessageView<'a> {
    /// The message's kind.
    pub status: Status,
    /// The message's text.
    pub text: &'a str,
    /// How deeply the message is nested: 0 for a top-level message, 1 for its children.
    pub depth: usize,
}

impl<'a> MessageView<'a> {
    /// A view of a message line.
    pub fn new(status: Status, text: &'a str, depth: usize) -> Self {
        MessageView {
            status,
            text,
            depth,
        }
    }
}

/// A status message, printed as one line, with any nested messages indented under it.
///
/// Works with no setup; the app theme draws it and [`Options`](crate::Options) decide
/// the stream (stderr by default).
///
/// ```rust,no_run
/// use paloma::Message;
///
/// # fn main() -> paloma::Result {
/// paloma::error("Port 3000 is already in use")
///     .nest(Message::hint("Stop the other server or pick another port"))
///     .print()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Message<'a> {
    status: Status,
    text: String,
    children: Vec<Message<'a>>,
    stream: Option<Stream>,
    theme: Option<&'a dyn Theme>,
}

macro_rules! constructors {
    ($($(#[$meta:meta])* $name:ident => $status:ident),*) => {
        impl Message<'_> {
            $(
                $(#[$meta])*
                pub fn $name(text: impl Into<String>) -> Self {
                    Self::new(Status::$status, text)
                }
            )*
        }

        $(
            $(#[$meta])*
            pub fn $name(text: impl Into<String>) -> Message<'static> {
                Message::new(Status::$status, text)
            }
        )*
    };
}

constructors! {
    /// A description message.
    description => Description,
    /// A hint message.
    hint => Hint,
    /// An info message.
    info => Info,
    /// A success message.
    success => Success,
    /// A warning message.
    warning => Warning,
    /// An error message.
    error => Error
}

impl<'a> Message<'a> {
    /// A message of any status.
    pub fn new(status: Status, text: impl Into<String>) -> Self {
        Message {
            status,
            text: text.into(),
            children: Vec::new(),
            stream: None,
            theme: None,
        }
    }

    /// Adds a message drawn indented under this one. Nested messages can nest too.
    pub fn nest(mut self, child: Message<'a>) -> Self {
        self.children.push(child);
        self
    }

    /// Where [`print`](Self::print) writes. Left unset, the app options decide
    /// ([`Options::messages`](crate::Options::messages) and the group of this status),
    /// then stderr.
    pub fn stream(mut self, stream: Stream) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Draws this message with `theme` instead of the app theme set with
    /// [`set_theme`](crate::set_theme).
    pub fn theme(mut self, theme: &'a dyn Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// The message's status.
    pub fn status(&self) -> Status {
        self.status
    }

    /// The message's text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The nested messages.
    pub fn children(&self) -> &[Message<'a>] {
        &self.children
    }

    /// The stream this message prints to.
    fn resolved_stream(&self) -> Stream {
        let options = crate::options::current();
        self.stream
            .or(options.status(self.status).stream)
            .or(options.messages.stream)
            .unwrap_or_default()
    }

    /// Prints the message and its nested messages.
    pub fn print(self) -> Result {
        let stream = self.resolved_stream();
        self.write(&stream.term(), stream)
    }

    /// Like [`print`](Self::print) but writes to `term`.
    pub fn print_on(self, term: &Term) -> Result {
        let stream = self.resolved_stream();
        self.write(term, stream)
    }

    fn write(&self, term: &Term, stream: Stream) -> Result {
        let mut out = String::new();
        let app_theme;
        let theme: &dyn Theme = match self.theme {
            Some(theme) => theme,
            None => {
                app_theme = app::current();
                &*app_theme
            }
        };
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, stream);
        self.render(&mut out, theme, &ctx, 0)
            .map_err(std::io::Error::other)?;
        term.write_line(&out)?;
        term.flush()?;
        Ok(())
    }

    /// Draws this message and its children, one line each, without a final newline.
    pub(crate) fn render(
        &self,
        f: &mut dyn std::fmt::Write,
        theme: &dyn Theme,
        ctx: &Context,
        depth: usize,
    ) -> std::fmt::Result {
        theme.format_message(f, ctx, &MessageView::new(self.status, &self.text, depth))?;
        for child in &self.children {
            writeln!(f)?;
            child.render(f, theme, ctx, depth + 1)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{SimpleTheme, Tokens};

    fn plain(message: &Message) -> String {
        let tokens = Tokens::plain();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let mut out = String::new();
        message.render(&mut out, &SimpleTheme, &ctx, 0).unwrap();
        out
    }

    #[test]
    fn nested_messages_indent_per_level() {
        let message = error("Port 3000 is already in use").nest(
            Message::hint("Stop the other server").nest(Message::description("or pick a port")),
        );
        assert_eq!(
            plain(&message),
            "error: Port 3000 is already in use\n  hint: Stop the other server\n    or pick a port"
        );
    }

    #[test]
    fn shortcuts_match_constructors() {
        assert_eq!(warning("x").status(), Message::warning("x").status());
        assert_eq!(success("ok").text(), "ok");
    }
}
