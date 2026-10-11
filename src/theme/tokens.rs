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
    /// Before a section title.
    section,
    /// Box border, horizontal.
    box_h,
    /// Box border, vertical.
    box_v,
    /// Box corner, top left.
    box_tl,
    /// Box corner, top right.
    box_tr,
    /// Box corner, bottom left.
    box_bl,
    /// Box corner, bottom right.
    box_br,
}

impl Glyphs {
    /// No glyphs: messages carry words instead (`error: …`).
    pub fn none() -> Self {
        Self::default()
    }

    /// Unicode symbols.
    pub fn unicode() -> Self {
        Self::none()
            .section("#")
            .box_h("─")
            .box_v("│")
            .box_tl("╭")
            .box_tr("╮")
            .box_bl("╰")
            .box_br("╯")
            .hint("›")
            .info("•")
            .success("✔")
            .warning("▲")
            .error("✘")
    }

    /// ASCII fallbacks for terminals without Unicode (see
    /// [`Capabilities::unicode`](crate::terminal::Capabilities::unicode)).
    pub fn ascii() -> Self {
        Self::ascii_frames()
            .hint(">")
            .info("*")
            .success("+")
            .warning("!")
            .error("x")
    }

    /// Only the ASCII section mark and box parts, no status glyphs.
    pub fn ascii_frames() -> Self {
        Self::none()
            .section("#")
            .box_h("-")
            .box_v("|")
            .box_tl("+")
            .box_tr("+")
            .box_bl("+")
            .box_br("+")
    }

    /// A glyph by name, for theme hooks: see [`GlyphName`].
    pub fn get(&self, name: GlyphName) -> &'static str {
        match name {
            GlyphName::Section => self.section,
            GlyphName::BoxH => self.box_h,
            GlyphName::BoxV => self.box_v,
            GlyphName::BoxTl => self.box_tl,
            GlyphName::BoxTr => self.box_tr,
            GlyphName::BoxBl => self.box_bl,
            GlyphName::BoxBr => self.box_br,
        }
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

/// The structural glyphs of [`Glyphs`] (status glyphs are read with
/// [`Glyphs::status`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GlyphName {
    /// Before a section title.
    Section,
    /// Box border, horizontal.
    BoxH,
    /// Box border, vertical.
    BoxV,
    /// Box corner, top left.
    BoxTl,
    /// Box corner, top right.
    BoxTr,
    /// Box corner, bottom left.
    BoxBl,
    /// Box corner, bottom right.
    BoxBr,
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

settings! {
    /// How an output block looks. Unset fields fall back to the family base, then to
    /// the built-in defaults and the palette roles.
    pub struct BlockStyle {
        /// Blank lines printed before the block. Default 0.
        space_before: usize,
        /// Columns between a key and its value, or between table columns. Default 3.
        gap: usize,
        /// Narrowest panel. Default 40.
        min_width: usize,
        /// Widest panel. Default 64.
        max_width: usize,
        /// Section and panel titles. Default: `text` (sections), `accent_strong`
        /// (panels); the colorful preset makes them bold.
        title_style: Style,
        /// Keys of key/value rows. Default: `muted`.
        key_style: Style,
        /// Values of key/value rows. Default: `text`.
        value_style: Style,
        /// Table headers. Default: `text`; the colorful preset makes them bold.
        header_style: Style,
        /// Table cells and panel rows. Default: `text`.
        cell_style: Style,
        /// Box borders and the section mark's trail. Default: `faint`; the section mark
        /// itself uses `accent`.
        border_style: Style,
        /// Right-aligned panel values. Default: `subtle`.
        trailing_style: Style,
    }
}

/// Which output block a style is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlockKind {
    /// [`Section`](crate::Section).
    Section,
    /// [`KeyValue`](crate::KeyValue).
    KeyValue,
    /// [`Table`](crate::Table).
    Table,
    /// [`Panel`](crate::Panel).
    Panel,
}

/// Component tokens for output blocks: a base for every block, and one group per
/// block that overrides it.
#[derive(Clone, Debug, Default)]
pub struct BlockTokens {
    base: BlockStyle,
    section: BlockStyle,
    key_value: BlockStyle,
    table: BlockStyle,
    panel: BlockStyle,
}

macro_rules! block_groups {
    ($($(#[$meta:meta])* $group:ident),*) => {
        impl BlockTokens {
            $(
                $(#[$meta])*
                pub fn $group(mut self, f: impl FnOnce(BlockStyle) -> BlockStyle) -> Self {
                    self.$group = f(self.$group);
                    self
                }
            )*
        }
    };
}

block_groups! {
    /// Settings for every block.
    base,
    /// Sections.
    section,
    /// Key/value rows.
    key_value,
    /// Tables.
    table,
    /// Panels.
    panel
}

/// The final look of one block, after the fallbacks.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct BlockLook {
    /// Blank lines before the block.
    pub space_before: usize,
    /// Columns between keys and values, or between table columns.
    pub gap: usize,
    /// Narrowest panel.
    pub min_width: usize,
    /// Widest panel.
    pub max_width: usize,
    /// Titles.
    pub title_style: Style,
    /// Keys.
    pub key_style: Style,
    /// Values.
    pub value_style: Style,
    /// Table headers.
    pub header_style: Style,
    /// Cells and panel rows.
    pub cell_style: Style,
    /// Borders.
    pub border_style: Style,
    /// Right-aligned panel values.
    pub trailing_style: Style,
}

impl BlockTokens {
    /// Resolves the look of `kind`: its own group, the base, then the defaults.
    pub fn resolve(&self, kind: BlockKind, tokens: &Tokens) -> BlockLook {
        let member = match kind {
            BlockKind::Section => &self.section,
            BlockKind::KeyValue => &self.key_value,
            BlockKind::Table => &self.table,
            BlockKind::Panel => &self.panel,
        };
        let s = member.merged(&self.base);
        let role = |r: Role| tokens.palette.get(r).clone();
        BlockLook {
            space_before: s.space_before.unwrap_or(0),
            gap: s.gap.unwrap_or(3),
            min_width: s.min_width.unwrap_or(40),
            max_width: s.max_width.unwrap_or(64),
            title_style: s.title_style.unwrap_or_else(|| match kind {
                BlockKind::Panel => role(Role::accent_strong),
                _ => role(Role::text),
            }),
            key_style: s.key_style.unwrap_or_else(|| role(Role::muted)),
            value_style: s.value_style.unwrap_or_else(|| role(Role::text)),
            header_style: s.header_style.unwrap_or_else(|| role(Role::text)),
            cell_style: s.cell_style.unwrap_or_else(|| role(Role::text)),
            border_style: s.border_style.unwrap_or_else(|| role(Role::faint)),
            trailing_style: s.trailing_style.unwrap_or_else(|| role(Role::subtle)),
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
    blocks: BlockTokens,
}

impl Tokens {
    /// No colour, no status glyphs (`#` and ASCII boxes only), words before messages (`error: …`, `warning: …`), like
    /// rustc and cargo. The tokens of [`SimpleTheme`](crate::theme::SimpleTheme).
    pub fn plain() -> Self {
        Tokens {
            palette: Palette::plain(),
            glyphs: Glyphs::ascii_frames(),
            layout: Layout::default(),
            blocks: BlockTokens::default(),
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
            blocks: BlockTokens::default()
                .section(|s| s.title_style(Style::new().bold()))
                .table(|s| s.header_style(Style::new().bold()))
                .panel(|s| s.title_style(Style::new().cyan().bold())),
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

    /// Changes the output block tokens.
    pub fn blocks(mut self, f: impl FnOnce(BlockTokens) -> BlockTokens) -> Self {
        self.blocks = f(self.blocks);
        self
    }

    /// The output block tokens.
    pub fn get_blocks(&self) -> &BlockTokens {
        &self.blocks
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
