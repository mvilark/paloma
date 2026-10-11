use std::borrow::Cow;

use crate::theme::{Theme, Tokens};

/// A colourful theme: Unicode glyphs and colour, drawn with
/// [`Tokens::colorful`](crate::theme::Tokens::colorful).
///
/// Change any part of its look with [`restyle`](Theme::restyle):
///
/// ```rust
/// use paloma::{console::Style, theme::{ColorfulTheme, Theme}};
///
/// let theme = ColorfulTheme
///     .restyle(|t| t.prompts(|p| p.base(|b| b.value_style(Style::new().yellow().dim()))));
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct ColorfulTheme;

impl Theme for ColorfulTheme {
    fn tokens(&self) -> Cow<'_, Tokens> {
        Cow::Owned(Tokens::colorful())
    }
}
