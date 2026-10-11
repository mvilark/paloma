use crate::theme::Theme;

/// The default theme: no colour and ASCII only, drawn with
/// [`Tokens::plain`](crate::theme::Tokens::plain) (`Name: Ada`, `error: …`).
#[derive(Clone, Copy, Debug, Default)]
pub struct SimpleTheme;

impl Theme for SimpleTheme {}
