//! What a theme hook knows besides the component's own data.
use std::fmt::Display;

use console::{Style, StyledObject};

use crate::{
    terminal::{self, Capabilities, Stream},
    theme::Tokens,
};

/// Passed to theme hooks: the tokens to draw with, the stream the output goes to, and
/// what that stream's terminal can show.
///
/// Hooks read tokens from here, not from the theme, so a theme wrapped with
/// [`restyle`](crate::theme::Theme::restyle) draws with the new tokens.
pub struct Context<'a> {
    tokens: &'a Tokens,
    stream: Stream,
    capabilities: Capabilities,
}

impl<'a> Context<'a> {
    /// A context for drawing with `tokens` to `stream`.
    pub fn new(tokens: &'a Tokens, stream: Stream) -> Self {
        Context {
            tokens,
            stream,
            capabilities: terminal::capabilities(stream),
        }
    }

    /// The tokens to draw with.
    pub fn tokens(&self) -> &Tokens {
        self.tokens
    }

    /// The stream the output goes to.
    pub fn stream(&self) -> Stream {
        self.stream
    }

    /// What the stream's terminal can show.
    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    /// Applies `style` to `text` for this context's stream. `console` decides per
    /// stream whether colours are shown, so always style through here.
    pub fn paint<D: Display>(&self, style: &Style, text: D) -> StyledObject<D> {
        match self.stream {
            Stream::Stderr => style.clone().for_stderr().apply_to(text),
            Stream::Stdout => style.clone().for_stdout().apply_to(text),
        }
    }
}
