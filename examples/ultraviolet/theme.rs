//! Ultraviolet palette, glyphs and the `Theme` implementation.
use std::{borrow::Cow, fmt, sync::OnceLock};

use console::{Color, Style};
use paloma::theme::{
    BlockTokens, ConfirmButtonsView, ConfirmPromptView, Context, CursorShape,
    FuzzySelectPromptView, InputPromptView, LiveTokens, MessageTokens, PromptKind, PromptLook,
    PromptTokens, PromptView, Theme, Tokens,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    True,
    C256,
    C16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    Fg,
    Muted,
    Subtle,
    Faint,
    Accent,
    Strong,
    /// Decoration only (spinner trail, gradient tails); the example has no use for it yet.
    #[allow(dead_code)]
    Dim,
    Success,
    Warning,
    Error,
}

/// Every colour token of the guide as a `console::Style`.
#[derive(Clone)]
pub struct Palette {
    dark: bool,
    depth: Depth,
}

impl Palette {
    pub fn new(dark: bool, depth: Depth) -> Self {
        Palette { dark, depth }
    }

    pub fn style(&self, tok: Tok) -> Style {
        use Tok::*;
        let (rgb, c256): ((u8, u8, u8), u8) = match (tok, self.dark) {
            (Fg, true) => ((0xe8, 0xe6, 0xf0), 254),
            (Fg, false) => ((0x1d, 0x1a, 0x2b), 234),
            (Muted, true) => ((0xa9, 0xa4, 0xbd), 146),
            (Muted, false) => ((0x4d, 0x47, 0x66), 60),
            (Subtle, true) => ((0x7d, 0x78, 0x94), 103),
            (Subtle, false) => ((0x6b, 0x65, 0x85), 60),
            (Faint, true) => ((0x4a, 0x46, 0x60), 60),
            (Faint, false) => ((0xcf, 0xc9, 0xe6), 189),
            (Accent, true) => ((0xa3, 0x82, 0xff), 141),
            (Accent, false) => ((0x6d, 0x4a, 0xff), 99),
            (Strong, true) => ((0xd5, 0xc6, 0xff), 189),
            (Strong, false) => ((0x4b, 0x2f, 0xd0), 55),
            (Dim, true) => ((0x5a, 0x3f, 0xd0), 61),
            (Dim, false) => ((0xb9, 0xa6, 0xff), 147),
            (Success, true) => ((0x5e, 0xe0, 0xc8), 79),
            (Success, false) => ((0x0f, 0x7f, 0x6e), 30),
            (Warning, true) => ((0xf5, 0xc2, 0x6b), 222),
            (Warning, false) => ((0x9a, 0x5a, 0x00), 94),
            (Error, true) => ((0xff, 0x7f, 0x9e), 211),
            (Error, false) => ((0xc4, 0x24, 0x4f), 161),
        };
        match self.depth {
            Depth::True => Style::new().fg(Color::TrueColor(rgb.0, rgb.1, rgb.2)),
            Depth::C256 => Style::new().fg(Color::Color256(c256)),
            Depth::C16 => match tok {
                Fg | Muted => Style::new(),
                Subtle => Style::new().dim(),
                Faint => Style::new().black().bright(),
                Accent if self.dark => Style::new().magenta().bright(),
                Accent => Style::new().magenta(),
                Strong => Style::new().magenta().bright().bold(),
                Dim => Style::new().magenta().dim(),
                Success => Style::new().cyan(),
                Warning => Style::new().yellow(),
                Error => Style::new().red(),
            },
        }
    }

    /// Text in a token, styled for stderr (the icon and the progress bar).
    pub fn e(&self, tok: Tok, text: impl fmt::Display) -> String {
        self.style(tok).for_stderr().apply_to(text).to_string()
    }

    /// The wordmark gradient's end points (`wordmark-from`, `wordmark-to`).
    pub fn wordmark(&self) -> ((u8, u8, u8), (u8, u8, u8)) {
        if self.dark {
            ((0xd5, 0xc6, 0xff), (0x6d, 0x4a, 0xff))
        } else {
            ((0x4b, 0x2f, 0xd0), (0x8c, 0x6b, 0xff))
        }
    }
}

/// Unicode glyphs with their ASCII fallbacks.
#[derive(Clone, Copy)]
pub struct Glyphs {
    /// The ASCII fallback set is in use.
    pub ascii: bool,
    /// Neutral status (info rows).
    pub info: &'static str,
    pub active: &'static str,
    pub answered: &'static str,
    pub caret: &'static str,
    pub dot: &'static str,
    pub focus: &'static str,
    pub checked: &'static str,
    pub unchecked: &'static str,
    pub chosen: &'static str,
    pub pending: &'static str,
    pub spinner: &'static [&'static str],
    pub ok: &'static str,
    pub warn: &'static str,
    pub err: &'static str,
    pub nested: &'static str,
    pub bar: &'static str,
    pub cursor: &'static str,
    pub box_h: &'static str,
    pub box_v: &'static str,
    pub box_tl: &'static str,
    pub box_tr: &'static str,
    pub box_bl: &'static str,
    pub box_br: &'static str,
}

impl Glyphs {
    pub fn new(ascii: bool) -> Self {
        if ascii {
            Glyphs {
                ascii: true,
                info: "*",
                active: "*",
                answered: "o",
                caret: ">",
                dot: "*",
                focus: ">",
                checked: "[x]",
                unchecked: "[ ]",
                chosen: "(*)",
                pending: "( )",
                spinner: &["-", "\\", "|", "/"],
                ok: "+",
                warn: "!",
                err: "x",
                nested: "-",
                bar: "=",
                cursor: "_",
                box_h: "-",
                box_v: "|",
                box_tl: "+",
                box_tr: "+",
                box_bl: "+",
                box_br: "+",
            }
        } else {
            Glyphs {
                ascii: false,
                info: "•",
                active: "◆",
                answered: "◇",
                caret: "›",
                dot: "•",
                focus: "▸",
                checked: "■",
                unchecked: "□",
                chosen: "●",
                pending: "○",
                spinner: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
                ok: "✓",
                warn: "▲",
                err: "✕",
                nested: "↳",
                bar: "━",
                cursor: "█",
                box_h: "─",
                box_v: "│",
                box_tl: "╭",
                box_tr: "╮",
                box_bl: "╰",
                box_br: "╯",
            }
        }
    }
}

/// Everything decided at startup.
#[derive(Clone)]
pub struct Config {
    pub pal: Palette,
    pub g: Glyphs,
    pub motion: bool,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn init(config: Config) {
    let _ = CONFIG.set(config);
}

pub fn cfg() -> &'static Config {
    CONFIG.get().expect("theme config is set at startup")
}

/// The theme the app runs with, for drawing lines the way a prompt would.
pub fn app_theme() -> UltravioletTheme {
    UltravioletTheme::new(cfg().clone())
}

/// The Ultraviolet design system as paloma tokens, plus the hooks whose layout the
/// tokens cannot express: the step counter after the label, the input on its own line,
/// and the confirm choices and buttons.
pub struct UltravioletTheme {
    cfg: Config,
}

impl UltravioletTheme {
    pub fn new(cfg: Config) -> Self {
        UltravioletTheme { cfg }
    }

    /// `◆ Label  01/05`: the open prompt's mark and question, then the flow step the
    /// view carries (`step_style`, `step_gap` spaces after the label).
    fn label(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        look: &PromptLook,
        prompt: &str,
        step: Option<(usize, usize)>,
    ) -> fmt::Result {
        write!(
            f,
            "{} {}",
            ctx.paint(&look.mark_style, look.mark),
            ctx.paint(&look.label_style, prompt)
        )?;
        match step {
            None => Ok(()),
            Some((n, total)) => write!(
                f,
                "{}{}",
                " ".repeat(look.step_gap),
                ctx.paint(&look.step_style, format!("{n:02}/{total:02}"))
            ),
        }
    }
}

fn look(ctx: &Context, kind: PromptKind) -> PromptLook {
    ctx.tokens().get_prompts().resolve(kind, ctx.tokens())
}

impl Theme for UltravioletTheme {
    fn tokens(&self) -> Cow<'_, Tokens> {
        let p = &self.cfg.pal;
        let g = &self.cfg.g;
        let palette = paloma::theme::Palette::plain()
            .text(p.style(Tok::Fg))
            .muted(p.style(Tok::Muted))
            .subtle(p.style(Tok::Subtle))
            .faint(p.style(Tok::Faint))
            .accent(p.style(Tok::Accent))
            .accent_strong(p.style(Tok::Strong))
            .success(p.style(Tok::Success))
            .info(p.style(Tok::Accent))
            .warning(p.style(Tok::Warning))
            .error(p.style(Tok::Error));
        let glyphs = paloma::theme::Glyphs::none()
            .info(g.info)
            .success(g.ok)
            .warning(g.warn)
            .error(g.err)
            .section("#")
            .nested(g.nested)
            .spinner(g.spinner)
            .box_h(g.box_h)
            .box_v(g.box_v)
            .box_tl(g.box_tl)
            .box_tr(g.box_tr)
            .box_bl(g.box_bl)
            .box_br(g.box_br)
            .prompt(g.active)
            .answered(g.answered)
            .answered_separator("·")
            .focus(g.focus)
            .checked(g.checked)
            .unchecked(g.unchecked)
            .cursor(g.cursor)
            .mask(g.dot);
        let prompts = PromptTokens::default()
            .base(|b| {
                b.label_style(p.style(Tok::Fg).bold())
                    .answered_mark_style(p.style(Tok::Subtle))
                    .answered_label_style(p.style(Tok::Muted))
                    .active_style(p.style(Tok::Strong))
                    .item_style(p.style(Tok::Muted))
                    .match_style(p.style(Tok::Accent).bold())
                    .empty_style(p.style(Tok::Subtle).italic())
                    .list_separator_style(p.style(Tok::Fg))
                    .paging_style(p.style(Tok::Subtle))
                    // The `01/05` flow step counter: `fg-subtle`, two spaces after the label.
                    .step_style(p.style(Tok::Subtle))
                    .step_gap(2)
                    .empty_value("none")
                    .cancelled_value("cancelled")
                    .cancelled_style(p.style(Tok::Subtle).italic())
                    .legend_key_style(p.style(Tok::Muted))
                    .legend_style(p.style(Tok::Subtle).italic())
                    .disabled_style(p.style(Tok::Muted).strikethrough())
                    .item_description_style(p.style(Tok::Subtle).italic())
                    .item_indent(2)
                    .yes("Yes")
                    .no("No")
                    .choices("[Y]es / [N]o")
                    .choices_yes("[Y]es / [n]o")
                    .choices_no("[y]es / [N]o")
                    .yes_button("[Y]es")
                    .no_button("[N]o")
                    .button_style(p.style(Tok::Muted))
                    .focused_button_style(p.style(Tok::Accent).bold())
                    .suffix(g.caret)
                    // The row under a prompt sits at `col-2`; the caret turns `error`
                    // while it shows a validation error.
                    .message_indent(2)
                    .invalid_suffix_style(p.style(Tok::Error))
            })
            .confirm(|c| c.empty_value("Skipped"))
            // Typed text is drawn unstyled, after a blinking block cursor.
            .input(|i| {
                i.text_style(Style::new())
                    .cursor_shape(CursorShape::Block)
                    .cursor_blink(true)
            })
            .password(|pw| {
                pw.text_style(Style::new())
                    .cursor_shape(CursorShape::Block)
                    .cursor_blink(true)
            })
            .sort(|s| s.checked(g.chosen).unchecked(g.pending));
        // The hint nests under the message it belongs to (depth 1, so the layout indent
        // of 2): `Hint` in italic `fg-subtle`, two spaces, then the command to paste.
        let messages = MessageTokens::default().hint(|h| {
            h.glyph(None)
                .word(Some("Hint"))
                .word_gap(2)
                .word_style(p.style(Tok::Subtle).italic())
                .text_style(p.style(Tok::Strong))
        });
        let blocks = BlockTokens::default()
            .section(|s| s.title_style(p.style(Tok::Fg).bold()).space_before(1))
            .key_value(|k| {
                k.key_style(p.style(Tok::Muted))
                    .value_style(p.style(Tok::Fg))
                    .gap(3)
            })
            .table(|t| {
                t.header_style(p.style(Tok::Fg).bold())
                    .cell_style(p.style(Tok::Fg))
                    .gap(3)
            })
            .panel(|b| {
                b.title_style(p.style(Tok::Strong).bold())
                    .border_style(p.style(Tok::Faint))
                    .trailing_style(p.style(Tok::Subtle))
                    .min_width(40)
                    .max_width(64)
                    .space_before(1)
            });
        let live = LiveTokens::default().task(|t| {
            t.spinner_style(p.style(Tok::Accent))
                .label_style(p.style(Tok::Muted))
                .step_style(p.style(Tok::Muted))
                .nested_style(p.style(Tok::Faint))
                .elapsed_style(p.style(Tok::Subtle))
                .step_indent(4)
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

    /// The label and step only; the paging counter in `fg-subtle`.
    fn format_prompt(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &PromptView,
    ) -> fmt::Result {
        let look = look(ctx, view.kind);
        self.label(f, ctx, &look, view.prompt, view.step)?;
        if let Some((page, pages)) = view.paging {
            write!(
                f,
                " {} ",
                ctx.paint(&look.paging_style, format!("[Page {page}/{pages}]"))
            )?;
        }
        Ok(())
    }

    /// The input sits on its own line, indented to `col-2`, after the caret (in `error`
    /// while the row shows a validation error).
    fn format_input_prompt(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &InputPromptView,
    ) -> fmt::Result {
        let look = look(ctx, view.kind);
        self.label(f, ctx, &look, view.prompt, view.step)?;
        if let Some(default) = view.default {
            write!(
                f,
                " {}",
                ctx.paint(&look.hint_style, format!("({default})"))
            )?;
        }
        let caret = if view.invalid {
            &look.invalid_suffix_style
        } else {
            &look.mark_style
        };
        write!(f, "\n  {} ", ctx.paint(caret, look.suffix))
    }

    /// `◆ Label  [Y]es / [n]o`, or only the label above the buttons.
    fn format_confirm_prompt(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &ConfirmPromptView,
    ) -> fmt::Result {
        let look = look(ctx, PromptKind::Confirm);
        self.label(f, ctx, &look, view.prompt, view.step)?;
        if view.buttons {
            return Ok(());
        }
        let choices = match view.default {
            Some(true) => look.choices_yes,
            Some(false) => look.choices_no,
            None => look.choices,
        };
        write!(f, "  {} ", ctx.paint(&look.hint_style, choices))
    }

    /// `[Y]es / [N]o` under the label, indented to `col-2`; the focused one in `accent`,
    /// bold.
    fn format_confirm_buttons(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &ConfirmButtonsView,
    ) -> fmt::Result {
        let look = look(ctx, PromptKind::Confirm);
        let button = |label, focused| {
            let style = if focused {
                &look.focused_button_style
            } else {
                &look.button_style
            };
            ctx.paint(style, label)
        };
        write!(
            f,
            "  {} {} {}",
            button(look.yes_button, view.yes_focused),
            ctx.paint(&look.hint_style, "/"),
            button(look.no_button, !view.yes_focused)
        )
    }

    /// `◆ Label › search` with a block cursor in `accent`.
    fn format_fuzzy_select_prompt(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &FuzzySelectPromptView,
    ) -> fmt::Result {
        let look = look(ctx, PromptKind::FuzzySelect);
        self.label(f, ctx, &look, view.prompt, view.step)?;
        let (head, tail) = view.search.split_at(view.cursor);
        write!(
            f,
            " {} {}{}{}",
            ctx.paint(&look.mark_style, look.suffix),
            ctx.paint(&look.text_style, head),
            ctx.paint(&look.cursor_style, look.cursor),
            ctx.paint(&look.text_style, tail)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paloma::{
        terminal::Stream,
        theme::{
            Answer, AnsweredView, BlockKind, MessageSource, MessageView, PromptMessageView,
            SectionView, Status, TaskState, TaskStepView, TaskView,
        },
    };
    use std::time::Duration;

    fn plain(cfg: Config) -> UltravioletTheme {
        console::set_colors_enabled(false);
        console::set_colors_enabled_stderr(false);
        UltravioletTheme::new(cfg)
    }

    fn config(ascii: bool) -> Config {
        Config {
            pal: Palette::new(true, Depth::True),
            g: Glyphs::new(ascii),
            motion: false,
        }
    }

    /// The escape sequence a token produces, with styling forced on.
    fn code(dark: bool, depth: Depth, tok: Tok) -> String {
        Palette::new(dark, depth)
            .style(tok)
            .force_styling(true)
            .apply_to("x")
            .to_string()
    }

    #[test]
    fn palette_follows_depth_and_theme() {
        assert_eq!(
            code(true, Depth::True, Tok::Accent),
            "\x1b[38;2;163;130;255mx\x1b[0m"
        );
        assert_eq!(
            code(false, Depth::True, Tok::Accent),
            "\x1b[38;2;109;74;255mx\x1b[0m"
        );
        assert_eq!(
            code(true, Depth::C256, Tok::Accent),
            "\x1b[38;5;141mx\x1b[0m"
        );
        assert_eq!(
            code(false, Depth::C256, Tok::Accent),
            "\x1b[38;5;99mx\x1b[0m"
        );
        assert_eq!(code(true, Depth::C16, Tok::Accent), "\x1b[38;5;13mx\x1b[0m");
        assert_eq!(code(false, Depth::C16, Tok::Accent), "\x1b[35mx\x1b[0m");
    }

    #[test]
    fn ascii_glyphs_replace_unicode() {
        let g = Glyphs::new(true);
        assert_eq!((g.active, g.ok, g.warn, g.err), ("*", "+", "!", "x"));
        assert!(Glyphs::new(false).spinner.len() == 10);
    }

    #[test]
    fn answered_prompt_collapses_to_one_line() {
        let theme = plain(config(false));
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let mut out = String::new();
        let view = AnsweredView::new(PromptKind::Input, "Name", Answer::Text("Ada"));
        theme.format_answered(&mut out, &ctx, &view).unwrap();
        assert_eq!(out, "◇ Name · Ada");

        let theme = plain(config(true));
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let mut out = String::new();
        let view = PromptView::new(PromptKind::Select, "Name", None, false, None);
        theme.format_prompt(&mut out, &ctx, &view).unwrap();
        assert_eq!(out, "* Name");
    }

    #[test]
    fn the_flow_step_follows_the_label() {
        let theme = plain(config(false));
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let mut out = String::new();
        let view = PromptView::new(PromptKind::Select, "Protocol", None, false, Some((3, 5)));
        theme.format_prompt(&mut out, &ctx, &view).unwrap();
        assert_eq!(out, "◆ Protocol  03/05");
        let mut out = String::new();
        let view = InputPromptView::new(PromptKind::Input, "Email", None, false, Some((1, 5)));
        theme.format_input_prompt(&mut out, &ctx, &view).unwrap();
        assert_eq!(out, "◆ Email  01/05\n  › ");
    }

    #[test]
    fn the_caret_turns_error_and_the_row_is_indented() {
        let theme = plain(config(false));
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let look = look(&ctx, PromptKind::Input);
        let error = Palette::new(true, Depth::True).style(Tok::Error);
        assert_eq!((look.invalid_suffix_style, look.message_indent), (error, 2));
        assert_eq!(look.text_style, Style::new());
        assert_eq!(
            (look.cursor_shape, look.cursor_blink),
            (CursorShape::Block, true)
        );

        let mut line = String::new();
        let view = InputPromptView::new(PromptKind::Input, "Email", None, true, None);
        theme.format_input_prompt(&mut line, &ctx, &view).unwrap();
        assert_eq!(line, "◆ Email\n  › ");

        let mut row = String::new();
        let view = PromptMessageView::new(
            PromptKind::Input,
            Status::Error,
            "Enter a valid email",
            0,
            MessageSource::Validation,
        );
        theme.format_prompt_message(&mut row, &ctx, &view).unwrap();
        assert_eq!(row, "  ✕ Enter a valid email");
    }

    #[test]
    fn a_hint_nests_under_its_message() {
        let theme = plain(config(false));
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let mut out = String::new();
        let hint = MessageView::new(Status::Hint, "ultraviolet servers", 1);
        theme.format_message(&mut out, &ctx, &hint).unwrap();
        assert_eq!(out, "  Hint  ultraviolet servers");
    }

    #[test]
    fn blocks_and_tasks_follow_the_guide() {
        let theme = plain(config(false));
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let mut out = String::new();
        let section = SectionView::new("Setup");
        theme.format_section(&mut out, &ctx, &section).unwrap();
        assert_eq!(out, "# Setup");
        let blocks = tokens.get_blocks();
        let panel = blocks.resolve(BlockKind::Panel, &tokens);
        assert_eq!(
            (panel.space_before, panel.min_width, panel.max_width),
            (1, 40, 64)
        );
        assert_eq!(blocks.resolve(BlockKind::Section, &tokens).space_before, 1);
        assert_eq!(blocks.resolve(BlockKind::KeyValue, &tokens).space_before, 0);

        let mut step = String::new();
        theme
            .format_task_step(&mut step, &ctx, &TaskStepView::new("Opened tunnel"))
            .unwrap();
        assert_eq!(step, "    ↳ Opened tunnel");
        let mut done = String::new();
        let finished = TaskView::new(
            "Connected",
            TaskState::Finished(Status::Success),
            Duration::from_millis(1400),
            0,
        );
        theme.format_task(&mut done, &ctx, &finished).unwrap();
        assert_eq!(done, "✓ Connected  1.4s");
    }

    #[test]
    fn errors_carry_only_a_glyph() {
        let theme = plain(config(false));
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let mut out = String::new();
        let view = MessageView::new(Status::Error, "Enter a valid email", 0);
        theme.format_message(&mut out, &ctx, &view).unwrap();
        assert_eq!(out, "✕ Enter a valid email");
    }
}
