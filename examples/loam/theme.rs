//! Loam's look: a warm earth palette, floral glyphs, dotted rules. The whole design is
//! tokens: `LoamTheme` implements `Theme::tokens` and nothing else, so paloma draws
//! every prompt, message, block and spinner. See docs/design/loam.md.
use std::borrow::Cow;

use console::{Color, Style};
use paloma::{
    terminal::ColorDepth,
    theme::{
        BlockTokens, CursorShape, Glyphs, LiveTokens, MessageTokens, Palette, PromptTokens, Theme,
        Tokens,
    },
};

/// The colour roles of the design (docs/design/loam.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Parchment on dark, espresso on light.
    Text,
    /// Oat.
    Muted,
    /// Bark.
    Subtle,
    /// Soil: decoration only.
    Faint,
    /// Moss.
    Accent,
    /// Sprout.
    Strong,
    /// Leaf.
    Success,
    /// Rain.
    Info,
    /// Honey.
    Warning,
    /// Terracotta.
    Error,
}

/// Every tone of the design as a `console::Style`, for one palette (night or day) and
/// one colour depth.
#[derive(Clone, Copy, Debug)]
pub struct Colors {
    night: bool,
    depth: ColorDepth,
}

impl Colors {
    pub fn new(night: bool, depth: ColorDepth) -> Self {
        Colors { night, depth }
    }

    /// The 24-bit value and the nearest 256-colour index of a tone.
    fn values(&self, tone: Tone) -> ((u8, u8, u8), u8) {
        use Tone::*;
        match (tone, self.night) {
            (Text, true) => ((0xED, 0xE3, 0xD1), 254),
            (Text, false) => ((0x3B, 0x2F, 0x24), 236),
            (Muted, true) => ((0xB8, 0xA9, 0x92), 144),
            (Muted, false) => ((0x6B, 0x5B, 0x4A), 240),
            (Subtle, true) => ((0x8C, 0x7E, 0x6A), 101),
            (Subtle, false) => ((0x8E, 0x7D, 0x69), 102),
            (Faint, true) => ((0x5A, 0x50, 0x45), 239),
            (Faint, false) => ((0xD8, 0xCB, 0xB6), 187),
            (Accent, true) => ((0x9D, 0xBF, 0x7A), 107),
            (Accent, false) => ((0x5E, 0x80, 0x40), 64),
            (Strong, true) => ((0xC6, 0xE3, 0xA0), 193),
            (Strong, false) => ((0x3F, 0x61, 0x27), 22),
            (Success, true) => ((0xA3, 0xC9, 0x8F), 150),
            (Success, false) => ((0x4E, 0x7D, 0x4E), 65),
            (Info, true) => ((0x9E, 0xC3, 0xC9), 152),
            (Info, false) => ((0x4F, 0x7F, 0x88), 66),
            (Warning, true) => ((0xE8, 0xB8, 0x60), 179),
            (Warning, false) => ((0xA8, 0x6F, 0x12), 136),
            (Error, true) => ((0xD9, 0x78, 0x5B), 173),
            (Error, false) => ((0xB5, 0x50, 0x2F), 131),
        }
    }

    /// The style of `tone`: 24-bit, 256-colour or one of the 16 named colours.
    pub fn style(&self, tone: Tone) -> Style {
        use Tone::*;
        let ((r, g, b), c256) = self.values(tone);
        match self.depth {
            ColorDepth::TrueColor => Style::new().fg(Color::TrueColor(r, g, b)),
            ColorDepth::Ansi256 => Style::new().fg(Color::Color256(c256)),
            ColorDepth::Basic => match tone {
                Text | Muted => Style::new(),
                Subtle => Style::new().dim(),
                Faint => Style::new().black().bright(),
                Accent => Style::new().green(),
                Strong if self.night => Style::new().green().bright(),
                Strong => Style::new().green().bold(),
                Success => Style::new().green(),
                Info => Style::new().cyan(),
                Warning => Style::new().yellow(),
                Error => Style::new().red(),
            },
        }
    }
}

/// The two ends of the wordmark's gradient: moss to honey.
pub fn wordmark(night: bool) -> ((u8, u8, u8), (u8, u8, u8)) {
    let colors = Colors::new(night, ColorDepth::TrueColor);
    (
        colors.values(Tone::Accent).0,
        colors.values(Tone::Warning).0,
    )
}

/// The glyphs of the design, Unicode or ASCII.
#[derive(Clone, Debug)]
pub struct Set {
    pub prompt: &'static str,
    pub answered: &'static str,
    pub suffix: &'static str,
    pub separator: &'static str,
    pub focus: &'static str,
    pub checked: &'static str,
    pub unchecked: &'static str,
    pub cursor: &'static str,
    pub mask: &'static str,
    pub section: &'static str,
    pub nested: &'static str,
    pub info: &'static str,
    pub success: &'static str,
    pub warning: &'static str,
    pub error: &'static str,
    pub spinner: &'static [&'static str],
    pub rule_h: &'static str,
    pub rule_v: &'static str,
    pub corner: &'static str,
    pub button_open: &'static str,
    pub button_close: &'static str,
}

impl Set {
    pub fn new(ascii: bool) -> Self {
        if ascii {
            Set {
                prompt: "*",
                answered: "o",
                suffix: "~",
                separator: "-",
                focus: ">",
                checked: "(x)",
                unchecked: "( )",
                cursor: "_",
                mask: "*",
                section: "#",
                nested: "-",
                info: "i",
                success: "+",
                warning: "!",
                error: "x",
                spinner: &[".", "o", "O", "o"],
                rule_h: "-",
                rule_v: ":",
                corner: ".",
                button_open: "<",
                button_close: ">",
            }
        } else {
            Set {
                prompt: "✿",
                answered: "❀",
                suffix: "~",
                separator: "—",
                focus: "❧",
                checked: "◉",
                unchecked: "◌",
                cursor: "▏",
                mask: "⁎",
                section: "❦",
                nested: "⤷",
                info: "◦",
                success: "✓",
                warning: "☂",
                error: "✗",
                spinner: &["◜", "◝", "◞", "◟"],
                rule_h: "┈",
                rule_v: "┊",
                corner: "·",
                button_open: "‹",
                button_close: "›",
            }
        }
    }

    /// Every glyph, for checking them as a group.
    #[cfg(test)]
    fn all(&self) -> Vec<&'static str> {
        let mut all = vec![
            self.prompt,
            self.answered,
            self.suffix,
            self.separator,
            self.focus,
            self.checked,
            self.unchecked,
            self.cursor,
            self.mask,
            self.section,
            self.nested,
            self.info,
            self.success,
            self.warning,
            self.error,
            self.rule_h,
            self.rule_v,
            self.corner,
            self.button_open,
            self.button_close,
        ];
        all.extend(self.spinner);
        all
    }
}

/// Loam, as paloma tokens. It overrides no hook: paloma's default layouts, drawn with
/// these colours, glyphs and spacings, are the whole design.
pub struct LoamTheme {
    colors: Colors,
    set: Set,
}

impl LoamTheme {
    pub fn new(night: bool, depth: ColorDepth, ascii: bool) -> Self {
        LoamTheme {
            colors: Colors::new(night, depth),
            set: Set::new(ascii),
        }
    }
}

impl Theme for LoamTheme {
    fn tokens(&self) -> Cow<'_, Tokens> {
        use Tone::*;
        let s = |tone| self.colors.style(tone);
        let soft = |tone| self.colors.style(tone).italic();
        let g = &self.set;

        let palette = Palette::plain()
            .text(s(Text))
            .muted(s(Muted))
            .subtle(s(Subtle))
            .faint(s(Faint))
            .accent(s(Accent))
            .accent_strong(s(Strong))
            .success(s(Success))
            .info(s(Info))
            .warning(s(Warning))
            .error(s(Error));
        let glyphs = Glyphs::none()
            .info(g.info)
            .success(g.success)
            .warning(g.warning)
            .error(g.error)
            .section(g.section)
            .nested(g.nested)
            .spinner(g.spinner)
            .box_h(g.rule_h)
            .box_v(g.rule_v)
            .box_tl(g.corner)
            .box_tr(g.corner)
            .box_bl(g.corner)
            .box_br(g.corner)
            .prompt(g.prompt)
            .answered(g.answered)
            .prompt_suffix(g.suffix)
            .answered_separator(g.separator)
            .focus(g.focus)
            .checked(g.checked)
            .unchecked(g.unchecked)
            .cursor(g.cursor)
            .mask(g.mask);
        let prompts = PromptTokens::default().base(|b| {
            b
                // The question is never bold: Loam does not shout.
                .mark_style(s(Accent))
                .label_style(s(Text))
                .answered_mark_style(s(Subtle))
                .answered_label_style(s(Muted))
                .separator_style(s(Faint))
                .value_style(s(Text))
                .list_separator_style(s(Subtle))
                .empty_value("none")
                .item_style(s(Muted))
                .active_style(s(Strong))
                .focus_style(s(Accent))
                .checked_style(s(Accent))
                .unchecked_style(s(Subtle))
                .match_style(s(Strong).underlined())
                .cursor_style(s(Accent))
                .paging_style(s(Subtle))
                .hint_style(soft(Subtle))
                .placeholder_style(soft(Subtle))
                .item_description_style(soft(Subtle))
                .disabled_style(s(Faint).strikethrough())
                .legend_key_style(s(Muted))
                .legend_style(soft(Subtle))
                // The `03/07` counter stays: a format token (`3 of 7`) does not exist.
                .step_style(s(Subtle))
                .message_indent(2)
                .item_indent(2)
                .button_indent(2)
                .invalid_suffix_style(s(Error))
                .cursor_shape(CursorShape::Bar)
                .cursor_blink(true)
                .yes("yes")
                .no("no")
                .yes_button("yes")
                .no_button("no")
                .button_brackets((g.button_open, g.button_close))
                .button_style(s(Muted))
                .focused_button_style(s(Accent).bold())
        });
        let messages = MessageTokens::default()
            .description(|d| d.text_style(soft(Subtle)))
            .hint(|h| {
                h.glyph(None)
                    .word(Some("tip"))
                    .word_gap(2)
                    .word_style(soft(Subtle))
                    .text_style(soft(Subtle))
            });
        let blocks = BlockTokens::default()
            .section(|b| b.title_style(s(Strong)).space_before(1))
            .panel(|b| {
                b.title_style(s(Accent))
                    .border_style(s(Faint))
                    .trailing_style(s(Subtle))
                    .min_width(40)
                    .max_width(60)
                    .space_before(1)
            })
            .key_value(|b| b.key_style(s(Muted)).value_style(s(Text)))
            .table(|b| b.header_style(soft(Muted)).cell_style(s(Text)));
        let live = LiveTokens::default().task(|t| {
            t.spinner_style(s(Accent))
                .label_style(s(Muted))
                .step_style(s(Subtle))
                .nested_style(s(Faint))
                .elapsed_style(s(Faint))
                .step_indent(2)
        });

        Cow::Owned(
            Tokens::default()
                .palette(|_| palette)
                .glyphs(|_| glyphs)
                .messages(|_| messages)
                .blocks(|_| blocks)
                .live(|_| live)
                .prompts(|_| prompts),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paloma::{
        terminal::Stream,
        theme::{
            Answer, AnsweredView, BlockKind, Context, MessageView, PromptKind, SectionView, Status,
            TaskState, TaskStepView, TaskView,
        },
    };
    use std::time::Duration;

    const TONES: [Tone; 10] = [
        Tone::Text,
        Tone::Muted,
        Tone::Subtle,
        Tone::Faint,
        Tone::Accent,
        Tone::Strong,
        Tone::Success,
        Tone::Info,
        Tone::Warning,
        Tone::Error,
    ];

    fn plain(ascii: bool) -> LoamTheme {
        console::set_colors_enabled(false);
        console::set_colors_enabled_stderr(false);
        LoamTheme::new(true, ColorDepth::TrueColor, ascii)
    }

    #[test]
    fn every_unicode_glyph_is_one_column_wide() {
        for glyph in Set::new(false).all() {
            assert_eq!(console::measure_text_width(glyph), 1, "{glyph:?}");
        }
    }

    #[test]
    fn the_ascii_set_is_ascii() {
        for glyph in Set::new(true).all() {
            assert!(glyph.is_ascii(), "{glyph:?}");
        }
        assert_ne!(Set::new(true).all(), Set::new(false).all());
    }

    #[test]
    fn night_and_day_differ_on_every_role() {
        for depth in [ColorDepth::TrueColor, ColorDepth::Ansi256] {
            for tone in TONES {
                assert_ne!(
                    Colors::new(true, depth).style(tone),
                    Colors::new(false, depth).style(tone),
                    "{tone:?} at {depth:?}"
                );
            }
        }
    }

    #[test]
    fn the_palette_follows_the_depth() {
        let code = |night, depth, tone| {
            Colors::new(night, depth)
                .style(tone)
                .force_styling(true)
                .apply_to("x")
                .to_string()
        };
        assert_eq!(
            code(true, ColorDepth::TrueColor, Tone::Accent),
            "\x1b[38;2;157;191;122mx\x1b[0m"
        );
        assert_eq!(
            code(false, ColorDepth::Ansi256, Tone::Accent),
            "\x1b[38;5;64mx\x1b[0m"
        );
        assert_eq!(
            code(true, ColorDepth::Basic, Tone::Warning),
            "\x1b[33mx\x1b[0m"
        );
    }

    #[test]
    fn the_wordmark_runs_from_moss_to_honey() {
        assert_eq!(wordmark(true), ((0x9D, 0xBF, 0x7A), (0xE8, 0xB8, 0x60)));
        assert_eq!(wordmark(false), ((0x5E, 0x80, 0x40), (0xA8, 0x6F, 0x12)));
    }

    #[test]
    fn messages_are_glyphs_and_hints_are_tips() {
        let theme = plain(false);
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let draw = |status, depth| {
            let mut out = String::new();
            let view = MessageView::new(status, "frost tonight", depth);
            theme.format_message(&mut out, &ctx, &view).unwrap();
            out
        };
        assert_eq!(draw(Status::Warning, 0), "☂ frost tonight");
        assert_eq!(draw(Status::Error, 0), "✗ frost tonight");
        assert_eq!(draw(Status::Hint, 1), "  tip  frost tonight");
        assert_eq!(draw(Status::Description, 0), "frost tonight");
    }

    #[test]
    fn prompts_answer_in_one_floral_line() {
        let theme = plain(false);
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let mut out = String::new();
        let view = AnsweredView::new(PromptKind::Input, "name", Answer::Text("the back plot"));
        theme.format_answered(&mut out, &ctx, &view).unwrap();
        assert_eq!(out, "❀ name — the back plot");

        let theme = plain(true);
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let mut out = String::new();
        theme.format_answered(&mut out, &ctx, &view).unwrap();
        assert_eq!(out, "o name - the back plot");
    }

    #[test]
    fn blocks_are_airy_and_tasks_are_arcs() {
        let theme = plain(false);
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let blocks = tokens.get_blocks();
        assert_eq!(blocks.resolve(BlockKind::Section, &tokens).space_before, 1);
        assert_eq!(blocks.resolve(BlockKind::Panel, &tokens).space_before, 1);
        assert_eq!(blocks.resolve(BlockKind::Table, &tokens).space_before, 0);

        let mut out = String::new();
        theme
            .format_section(&mut out, &ctx, &SectionView::new("your garden"))
            .unwrap();
        assert_eq!(out, "❦ your garden");

        let mut out = String::new();
        let running = TaskView::new("testing the soil...", TaskState::Running, Duration::ZERO, 1);
        theme.format_task(&mut out, &ctx, &running).unwrap();
        assert_eq!(out, "◝ testing the soil...  0.0s");
        let mut out = String::new();
        theme
            .format_task_step(&mut out, &ctx, &TaskStepView::new("pH 6.4"))
            .unwrap();
        assert_eq!(out, "  ⤷ pH 6.4");
    }
}
