//! Design tokens: the values every component is drawn with.
//!
//! [`Tokens`] holds three semantic groups ([`Palette`], [`Glyphs`], [`Layout`]) and one
//! group of component tokens per component family ([`MessageTokens`]). Component tokens
//! are optional: anything left unset falls back from the member (one status) to the
//! family base, then to the semantic tokens. Changing a semantic token restyles every
//! component that has not overridden it.
//!
//! Every group uses the same closure-builder style as [`Options`](crate::Options):
//!
//! ```rust
//! use paloma::{console::Style, theme::Tokens};
//!
//! let tokens = Tokens::colorful()
//!     .palette(|p| p.accent(Style::new().magenta()))
//!     .glyphs(|g| g.warning("▲"))
//!     .messages(|m| m.error(|e| e.word(None)));
//! ```
use console::Style;

use crate::{output::message::Status, settings::settings};

macro_rules! roles {
    ($($(#[$meta:meta])* $role:ident),* $(,)?) => {
        /// Colour roles, as `console` styles. Components use roles, never raw colours,
        /// so a palette change reaches all of them.
        #[derive(Clone, Debug, Default)]
        pub struct Palette {
            $($role: Style,)*
        }

        impl Palette {
            $(
                $(#[$meta])*
                pub fn $role(mut self, style: Style) -> Self {
                    self.$role = style;
                    self
                }
            )*
        }

        /// Reading the palette, for theme hooks.
        impl Palette {
            /// The style of a role, by name. Each role also has a getter: [`Palette::get`]
            /// is for code that picks the role at run time.
            pub fn get(&self, role: Role) -> &Style {
                match role {
                    $(Role::$role => &self.$role,)*
                }
            }
        }

        /// A colour role of the [`Palette`].
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum Role {
            $($(#[$meta])* $role,)*
        }
    };
}

roles! {
    /// Answers, values and messages.
    text,
    /// Answered labels, unselected options.
    muted,
    /// Hints, counters, descriptions.
    subtle,
    /// Decoration only: rules, borders.
    faint,
    /// The live mark: the current item, the cursor.
    accent,
    /// Emphasis inside the accent.
    accent_strong,
    /// Something finished well.
    success,
    /// Neutral information.
    info,
    /// Something needs attention.
    warning,
    /// Something failed.
    error,
}

impl Palette {
    /// No colour at all.
    pub fn plain() -> Self {
        Self::default()
    }

    /// The colours of [`ColorfulTheme`](crate::theme::ColorfulTheme).
    pub fn colorful() -> Self {
        Self::plain()
            .subtle(Style::new().black().bright())
            .faint(Style::new().black().bright())
            .accent(Style::new().cyan())
            .accent_strong(Style::new().cyan().bold())
            .success(Style::new().green())
            .info(Style::new().blue())
            .warning(Style::new().yellow())
            .error(Style::new().red())
    }
}

macro_rules! glyphs {
    ($($(#[$meta:meta])* $glyph:ident),* $(,)?) => {
        /// Named symbols. An empty glyph means "no glyph".
        #[derive(Clone, Debug, Default)]
        pub struct Glyphs {
            $($glyph: &'static str,)*
        }

        impl Glyphs {
            $(
                $(#[$meta])*
                pub fn $glyph(mut self, glyph: &'static str) -> Self {
                    self.$glyph = glyph;
                    self
                }
            )*
        }
    };
}

glyphs! {
    /// Before a description (none by default).
    description,
    /// Before a hint.
    hint,
    /// Before an info message.
    info,
    /// Before a success message.
    success,
    /// Before a warning.
    warning,
    /// Before an error.
    error,
}

impl Glyphs {
    /// No glyphs: messages carry words instead (`error: …`).
    pub fn none() -> Self {
        Self::default()
    }

    /// Unicode symbols.
    pub fn unicode() -> Self {
        Self::none()
            .hint("›")
            .info("•")
            .success("✔")
            .warning("▲")
            .error("✘")
    }

    /// ASCII fallbacks for terminals without Unicode (see
    /// [`Capabilities::unicode`](crate::terminal::Capabilities::unicode)).
    pub fn ascii() -> Self {
        Self::none()
            .hint(">")
            .info("*")
            .success("+")
            .warning("!")
            .error("x")
    }

    /// The glyph for a status.
    pub fn status(&self, status: Status) -> &'static str {
        match status {
            Status::Description => self.description,
            Status::Hint => self.hint,
            Status::Info => self.info,
            Status::Success => self.success,
            Status::Warning => self.warning,
            Status::Error => self.error,
        }
    }
}

/// Spacing.
#[derive(Clone, Debug)]
pub struct Layout {
    indent: usize,
}

impl Default for Layout {
    fn default() -> Self {
        Layout { indent: 2 }
    }
}

impl Layout {
    /// Columns a nested line is indented by, per level. Default 2.
    pub fn indent(mut self, columns: usize) -> Self {
        self.indent = columns;
        self
    }

    /// See [`indent`](Self::indent).
    pub fn get_indent(&self) -> usize {
        self.indent
    }
}

settings! {
    /// How one kind of message looks. Unset fields fall back to the family base, then
    /// to the semantic tokens.
    pub struct MessageStyle {
        /// The glyph before the text; `None` for no glyph.
        glyph: Option<&'static str>,
        /// A word before the text, such as `error:`; `None` for no word.
        word: Option<&'static str>,
        /// The glyph's style.
        glyph_style: Style,
        /// The word's style.
        word_style: Style,
        /// The text's style.
        text_style: Style,
    }
}

/// Component tokens for messages: a base for every status, and one group per status
/// that overrides it.
#[derive(Clone, Debug, Default)]
pub struct MessageTokens {
    base: MessageStyle,
    description: MessageStyle,
    hint: MessageStyle,
    info: MessageStyle,
    success: MessageStyle,
    warning: MessageStyle,
    error: MessageStyle,
}

macro_rules! message_groups {
    ($($(#[$meta:meta])* $group:ident),*) => {
        impl MessageTokens {
            $(
                $(#[$meta])*
                pub fn $group(mut self, f: impl FnOnce(MessageStyle) -> MessageStyle) -> Self {
                    self.$group = f(self.$group);
                    self
                }
            )*
        }
    };
}

message_groups! {
    /// Settings for every status.
    base,
    /// Descriptions.
    description,
    /// Hints.
    hint,
    /// Info messages.
    info,
    /// Success messages.
    success,
    /// Warnings.
    warning,
    /// Errors.
    error
}

/// The final look of one message, after the fallbacks.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MessageLook {
    /// The glyph before the text, if any.
    pub glyph: Option<&'static str>,
    /// The word before the text, if any.
    pub word: Option<&'static str>,
    /// The glyph's style.
    pub glyph_style: Style,
    /// The word's style.
    pub word_style: Style,
    /// The text's style.
    pub text_style: Style,
}

impl MessageTokens {
    fn member(&self, status: Status) -> &MessageStyle {
        match status {
            Status::Description => &self.description,
            Status::Hint => &self.hint,
            Status::Info => &self.info,
            Status::Success => &self.success,
            Status::Warning => &self.warning,
            Status::Error => &self.error,
        }
    }

    /// Resolves the look of `status`: its own group, the base, then the semantic tokens.
    pub fn resolve(&self, status: Status, tokens: &Tokens) -> MessageLook {
        let style = self.member(status).merged(&self.base);
        let role = match status {
            Status::Description | Status::Hint => Role::subtle,
            Status::Info => Role::info,
            Status::Success => Role::success,
            Status::Warning => Role::warning,
            Status::Error => Role::error,
        };
        let role_style = tokens.palette.get(role).clone();
        let glyph = tokens.glyphs.status(status);
        MessageLook {
            glyph: style.glyph.unwrap_or((!glyph.is_empty()).then_some(glyph)),
            word: style.word.flatten(),
            glyph_style: style.glyph_style.unwrap_or_else(|| role_style.clone()),
            word_style: style.word_style.unwrap_or_else(|| role_style.clone()),
            text_style: style.text_style.unwrap_or_else(|| match status {
                Status::Description | Status::Hint => role_style,
                _ => tokens.palette.get(Role::text).clone(),
            }),
        }
    }
}

/// Every token a theme draws with.
#[derive(Clone, Debug, Default)]
pub struct Tokens {
    palette: Palette,
    glyphs: Glyphs,
    layout: Layout,
    messages: MessageTokens,
}

impl Tokens {
    /// No colour, no glyphs, words before messages (`error: …`, `warning: …`), like
    /// rustc and cargo. The tokens of [`SimpleTheme`](crate::theme::SimpleTheme).
    pub fn plain() -> Self {
        Tokens {
            palette: Palette::plain(),
            glyphs: Glyphs::none(),
            layout: Layout::default(),
            messages: MessageTokens::default()
                .hint(|s| s.word(Some("hint:")))
                .info(|s| s.word(Some("info:")))
                .success(|s| s.word(Some("success:")))
                .warning(|s| s.word(Some("warning:")))
                .error(|s| s.word(Some("error:"))),
        }
    }

    /// Colours and Unicode glyphs matching
    /// [`ColorfulTheme`](crate::theme::ColorfulTheme): an error is a red `✘` and red
    /// text.
    pub fn colorful() -> Self {
        Tokens {
            palette: Palette::colorful(),
            glyphs: Glyphs::unicode(),
            layout: Layout::default(),
            messages: MessageTokens::default().error(|s| s.text_style(Style::new().red())),
        }
    }

    /// Changes the palette.
    pub fn palette(mut self, f: impl FnOnce(Palette) -> Palette) -> Self {
        self.palette = f(self.palette);
        self
    }

    /// Changes the glyphs.
    pub fn glyphs(mut self, f: impl FnOnce(Glyphs) -> Glyphs) -> Self {
        self.glyphs = f(self.glyphs);
        self
    }

    /// Changes the layout.
    pub fn layout(mut self, f: impl FnOnce(Layout) -> Layout) -> Self {
        self.layout = f(self.layout);
        self
    }

    /// Changes the message tokens.
    pub fn messages(mut self, f: impl FnOnce(MessageTokens) -> MessageTokens) -> Self {
        self.messages = f(self.messages);
        self
    }

    /// The palette.
    pub fn get_palette(&self) -> &Palette {
        &self.palette
    }

    /// The glyphs.
    pub fn get_glyphs(&self) -> &Glyphs {
        &self.glyphs
    }

    /// The layout.
    pub fn get_layout(&self) -> &Layout {
        &self.layout
    }

    /// The message tokens.
    pub fn get_messages(&self) -> &MessageTokens {
        &self.messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_look_falls_back_from_status_to_base_to_semantic() {
        // Status group wins over the base; the base wins over the semantic tokens.
        let tokens = Tokens::colorful()
            .glyphs(|g| g.warning("!"))
            .messages(|m| m.base(|b| b.glyph(None)).error(|e| e.glyph(Some("E"))));
        let warning = tokens.messages.resolve(Status::Warning, &tokens);
        assert_eq!((warning.glyph, warning.word), (None, None));
        let error = tokens.messages.resolve(Status::Error, &tokens);
        assert_eq!(error.glyph, Some("E"));

        // Nothing set: the semantic glyph.
        let tokens = Tokens::colorful().glyphs(|g| g.warning("!"));
        let warning = tokens.messages.resolve(Status::Warning, &tokens);
        assert_eq!(warning.glyph, Some("!"));

        let plain = Tokens::plain();
        let hint = plain.messages.resolve(Status::Hint, &plain);
        assert_eq!((hint.glyph, hint.word), (None, Some("hint:")));
    }
}
