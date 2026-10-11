//! The default bodies of the [`Theme`](crate::theme::Theme) hooks that take a
//! [`Context`].
//!
//! They are public so a theme can wrap a default instead of copying it:
//!
//! ```rust
//! use std::fmt;
//! use paloma::theme::{defaults, Context, MessageView, Theme};
//!
//! struct Loud;
//!
//! impl Theme for Loud {
//!     fn format_message(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &MessageView) -> fmt::Result {
//!         defaults::format_message(f, ctx, view)?;
//!         write!(f, "!")
//!     }
//! }
//! ```
use std::fmt;

use crate::theme::{Context, MessageView};

/// One message line: the indent for its depth, then the glyph, the word and the text,
/// each styled by the resolved [`MessageLook`](crate::theme::MessageLook).
pub fn format_message(f: &mut dyn fmt::Write, ctx: &Context, view: &MessageView) -> fmt::Result {
    let tokens = ctx.tokens();
    let look = tokens.get_messages().resolve(view.status, tokens);
    write!(
        f,
        "{}",
        " ".repeat(view.depth * tokens.get_layout().get_indent())
    )?;
    if let Some(glyph) = look.glyph {
        write!(f, "{} ", ctx.paint(&look.glyph_style, glyph))?;
    }
    if let Some(word) = look.word {
        write!(f, "{} ", ctx.paint(&look.word_style, word))?;
    }
    write!(f, "{}", ctx.paint(&look.text_style, view.text))
}
