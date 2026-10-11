//! How everything paloma draws looks: the [`Theme`] trait, design [`Tokens`] and the
//! built-in themes.
//!
//! Three layers, from cheapest to most powerful: change the [`Tokens`] (with
//! [`Theme::restyle`]), change one component's tokens (for example
//! [`MessageTokens`]), or override a `format_*` hook.
use std::{borrow::Cow, fmt};

#[cfg(feature = "fuzzy-select")]
use console::style;
#[cfg(feature = "fuzzy-select")]
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

pub(crate) mod app;
mod colorful;
mod context;
pub mod defaults;
pub(crate) mod render;
mod restyle;
mod simple;
mod tokens;

pub use crate::output::{
    key_value::KeyValueView,
    message::{MessageView, Status},
    panel::PanelView,
    section::SectionView,
    table::TableView,
};
pub use colorful::ColorfulTheme;
pub use context::Context;
pub use restyle::Restyled;
pub use simple::SimpleTheme;
pub use tokens::{
    BlockKind, BlockLook, BlockStyle, BlockTokens, GlyphName, Glyphs, Layout, MessageLook,
    MessageStyle, MessageTokens, Palette, Role, Tokens,
};

/// Implements a theme for paloma.
pub trait Theme {
    /// The tokens this theme draws with. Defaults to [`Tokens::plain`].
    fn tokens(&self) -> Cow<'_, Tokens> {
        Cow::Owned(Tokens::plain())
    }

    /// The same theme drawn with other tokens: `f` gets this theme's tokens and returns
    /// the new ones. Works on any theme without implementing the trait.
    ///
    /// ```rust
    /// use paloma::theme::{ColorfulTheme, Theme};
    ///
    /// paloma::set_theme(ColorfulTheme::default().restyle(|t| t.glyphs(|g| g.warning("▲"))));
    /// ```
    fn restyle(self, f: impl FnOnce(Tokens) -> Tokens) -> Restyled<Self>
    where
        Self: Sized,
    {
        Restyled::new(self, f)
    }

    /// Formats one message line (see [`Message`](crate::Message)). The default is
    /// [`defaults::format_message`].
    fn format_message(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &MessageView,
    ) -> fmt::Result {
        defaults::format_message(f, ctx, view)
    }

    /// Formats a section title (see [`Section`](crate::Section)). The default is
    /// [`defaults::format_section`].
    fn format_section(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &SectionView,
    ) -> fmt::Result {
        defaults::format_section(f, ctx, view)
    }

    /// Formats key/value rows (see [`KeyValue`](crate::KeyValue)), one per line. The
    /// default is [`defaults::format_key_value`].
    fn format_key_value(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &KeyValueView,
    ) -> fmt::Result {
        defaults::format_key_value(f, ctx, view)
    }

    /// Formats a table (see [`Table`](crate::Table)). The default is
    /// [`defaults::format_table`].
    fn format_table(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &TableView) -> fmt::Result {
        defaults::format_table(f, ctx, view)
    }

    /// Formats a panel (see [`Panel`](crate::Panel)). The default is
    /// [`defaults::format_panel`].
    fn format_panel(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &PanelView) -> fmt::Result {
        defaults::format_panel(f, ctx, view)
    }

    /// Formats a prompt.
    #[inline]
    fn format_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        write!(f, "{}:", prompt)
    }

    /// Formats an input error (validation, parsing, password mismatch). The default
    /// draws an error [`Message`](crate::Message) line with this theme's tokens.
    fn format_error(&self, f: &mut dyn fmt::Write, err: &str) -> fmt::Result {
        let tokens = self.tokens();
        let ctx = Context::new(&tokens, crate::terminal::Stream::Stderr);
        self.format_message(f, &ctx, &MessageView::new(Status::Error, err, 0))
    }

    /// Formats a confirm prompt.
    fn format_confirm_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<bool>,
    ) -> fmt::Result {
        if !prompt.is_empty() {
            write!(f, "{} ", prompt)?;
        }
        match default {
            None => write!(f, "[y/n] ")?,
            Some(true) => write!(f, "[Y/n] ")?,
            Some(false) => write!(f, "[y/N] ")?,
        }
        Ok(())
    }

    /// Formats a confirm prompt without the hint ([y/n]).
    fn format_confirm_prompt_no_hint(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        if !prompt.is_empty() {
            write!(f, "{} ", prompt)?;
        }
        Ok(())
    }

    /// Formats one confirm button: `yes` tells which of the two it is, `focused`
    /// whether it is the one `Enter` would pick. Only used for
    /// [`Confirm::choices_as_buttons`](crate::Confirm::choices_as_buttons).
    fn format_confirm_button(
        &self,
        f: &mut dyn fmt::Write,
        yes: bool,
        focused: bool,
    ) -> fmt::Result {
        let label = if yes { "Yes" } else { "No" };
        if focused {
            write!(f, "[{label}]")
        } else {
            write!(f, " {label} ")
        }
    }

    /// Formats the row of confirm buttons; `yes_focused` says which one has focus.
    fn format_confirm_buttons(&self, f: &mut dyn fmt::Write, yes_focused: bool) -> fmt::Result {
        self.format_confirm_button(f, true, yes_focused)?;
        write!(f, "  ")?;
        self.format_confirm_button(f, false, !yes_focused)
    }

    /// Formats a confirm prompt after selection.
    fn format_confirm_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selection: Option<bool>,
    ) -> fmt::Result {
        let selection = selection.map(|b| if b { "yes" } else { "no" });

        match selection {
            Some(selection) if prompt.is_empty() => {
                write!(f, "{}", selection)
            }
            Some(selection) => {
                write!(f, "{} {}", prompt, selection)
            }
            None if prompt.is_empty() => Ok(()),
            None => {
                write!(f, "{}", prompt)
            }
        }
    }

    /// Formats an input prompt.
    fn format_input_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<&str>,
    ) -> fmt::Result {
        match default {
            Some(default) if prompt.is_empty() => write!(f, "[{}]: ", default),
            Some(default) => write!(f, "{} [{}]: ", prompt, default),
            None => write!(f, "{}: ", prompt),
        }
    }

    /// Formats an input prompt after selection.
    #[inline]
    fn format_input_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> fmt::Result {
        write!(f, "{}: {}", prompt, sel)
    }

    /// Formats a password prompt.
    #[inline]
    #[cfg(feature = "password")]
    fn format_password_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.format_input_prompt(f, prompt, None)
    }

    /// Formats a password prompt after selection.
    #[inline]
    #[cfg(feature = "password")]
    fn format_password_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
    ) -> fmt::Result {
        self.format_input_prompt_selection(f, prompt, "[hidden]")
    }

    /// The character echoed for each typed password character when the prompt masks
    /// its input (see [`Password::mask`](crate::Password::mask)). Defaults to `*`.
    #[cfg(feature = "password")]
    fn password_mask_glyph(&self) -> char {
        '*'
    }

    /// Formats a password prompt after selection, given how many mask characters the
    /// prompt asks to show (see [`Password::result_len`](crate::Password::result_len)).
    /// Defaults to [`format_password_prompt_selection`](Self::format_password_prompt_selection),
    /// which ignores the length.
    #[cfg(feature = "password")]
    fn format_password_prompt_selection_masked(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        _mask_len: usize,
    ) -> fmt::Result {
        self.format_password_prompt_selection(f, prompt)
    }

    /// Formats a select prompt.
    #[inline]
    fn format_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.format_prompt(f, prompt)
    }

    /// Formats a select prompt after selection.
    #[inline]
    fn format_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> fmt::Result {
        self.format_input_prompt_selection(f, prompt, sel)
    }

    /// Formats a multi select prompt.
    #[inline]
    fn format_multi_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.format_prompt(f, prompt)
    }

    /// Formats a sort prompt.
    #[inline]
    fn format_sort_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.format_prompt(f, prompt)
    }

    /// Formats a multi_select prompt after selection.
    fn format_multi_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        write!(f, "{}: ", prompt)?;
        for (idx, sel) in selections.iter().enumerate() {
            write!(f, "{}{}", if idx == 0 { "" } else { ", " }, sel)?;
        }
        Ok(())
    }

    /// Formats a sort prompt after selection.
    #[inline]
    fn format_sort_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        self.format_multi_select_prompt_selection(f, prompt, selections)
    }

    /// Formats a select prompt item.
    fn format_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        active: bool,
    ) -> fmt::Result {
        write!(f, "{} {}", if active { ">" } else { " " }, text)
    }

    /// Formats a multi select prompt item.
    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        write!(
            f,
            "{} {}",
            match (checked, active) {
                (true, true) => "> [x]",
                (true, false) => "  [x]",
                (false, true) => "> [ ]",
                (false, false) => "  [ ]",
            },
            text
        )
    }

    /// Formats a sort prompt item.
    fn format_sort_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        picked: bool,
        active: bool,
    ) -> fmt::Result {
        write!(
            f,
            "{} {}",
            match (picked, active) {
                (true, true) => "> [x]",
                (false, true) => "> [ ]",
                (_, false) => "  [ ]",
            },
            text
        )
    }

    /// Formats a fuzzy select prompt item.
    #[cfg(feature = "fuzzy-select")]
    fn format_fuzzy_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        active: bool,
        highlight_matches: bool,
        matcher: &SkimMatcherV2,
        search_term: &str,
    ) -> fmt::Result {
        write!(f, "{} ", if active { ">" } else { " " })?;

        if highlight_matches {
            if let Some((_score, indices)) = matcher.fuzzy_indices(text, search_term) {
                for (idx, c) in text.chars().enumerate() {
                    if indices.contains(&idx) {
                        write!(f, "{}", style(c).for_stderr().bold())?;
                    } else {
                        write!(f, "{}", c)?;
                    }
                }

                return Ok(());
            }
        }

        write!(f, "{}", text)
    }

    /// Formats a fuzzy select prompt.
    #[cfg(feature = "fuzzy-select")]
    fn format_fuzzy_select_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        search_term: &str,
        bytes_pos: usize,
    ) -> fmt::Result {
        if !prompt.is_empty() {
            write!(f, "{prompt} ")?;
        }

        let (st_head, st_tail) = search_term.split_at(bytes_pos);
        write!(f, "{st_head}|{st_tail}")
    }
}
