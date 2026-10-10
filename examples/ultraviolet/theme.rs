//! Ultraviolet palette, glyphs and the `Theme` implementation.
use std::{
    fmt,
    sync::{
        atomic::{AtomicU32, Ordering},
        OnceLock,
    },
};

use console::{Color, Style};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use paloma::theme::Theme;

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

    /// Text in a token, styled for stderr (prompts and progress).
    pub fn e(&self, tok: Tok, text: impl fmt::Display) -> String {
        self.style(tok).for_stderr().apply_to(text).to_string()
    }

    /// Text in a token, styled for stdout (results).
    pub fn o(&self, tok: Tok, text: impl fmt::Display) -> String {
        self.style(tok).apply_to(text).to_string()
    }

    /// Wordmark gradient: 24-bit only, otherwise `accent-strong`.
    pub fn gradient(&self, text: &str) -> String {
        let (from, to) = if self.dark {
            ((0xd5u8, 0xc6u8, 0xffu8), (0x6du8, 0x4au8, 0xffu8))
        } else {
            ((0x4b, 0x2f, 0xd0), (0x8c, 0x6b, 0xff))
        };
        let n = text.chars().count().max(2) - 1;
        text.chars()
            .enumerate()
            .map(|(i, c)| {
                let lerp =
                    |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * i as f32 / n as f32) as u8;
                let style = if self.depth == Depth::True {
                    Style::new().fg(Color::TrueColor(
                        lerp(from.0, to.0),
                        lerp(from.1, to.1),
                        lerp(from.2, to.2),
                    ))
                } else {
                    self.style(Tok::Strong)
                };
                style.bold().for_stderr().apply_to(c).to_string()
            })
            .collect()
    }
}

/// Unicode glyphs with their ASCII fallbacks.
#[derive(Clone, Copy)]
pub struct Glyphs {
    pub active: &'static str,
    pub answered: &'static str,
    pub caret: &'static str,
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
                active: "*",
                answered: "o",
                caret: ">",
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
                active: "◆",
                answered: "◇",
                caret: "›",
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

/// `fg-subtle` + italic: hints, `cancelled`, `skipped`.
fn hint(pal: &Palette, text: &str) -> String {
    pal.style(Tok::Subtle)
        .italic()
        .for_stderr()
        .apply_to(text)
        .to_string()
}

/// Current flow step as `n << 8 | total`, or 0 outside a flow. The `Theme` trait has no
/// notion of a flow (Gap 6), so the example hands the step to the theme out of band.
static STEP: AtomicU32 = AtomicU32::new(0);

pub fn set_step(n: u32, total: u32) {
    STEP.store(n << 8 | total, Ordering::Relaxed);
}

pub fn clear_step() {
    STEP.store(0, Ordering::Relaxed);
}

pub struct UltravioletTheme {
    cfg: Config,
}

impl UltravioletTheme {
    pub fn new(cfg: Config) -> Self {
        UltravioletTheme { cfg }
    }

    fn label(&self, f: &mut dyn fmt::Write, prompt: &str, glyph: &str, tok: Tok) -> fmt::Result {
        let p = &self.cfg.pal;
        write!(
            f,
            "{} {}",
            p.e(tok, glyph),
            p.style(Tok::Fg).bold().for_stderr().apply_to(prompt)
        )?;
        // `01/05` step counter: `fg-subtle`, regular weight, after the label.
        match STEP.load(Ordering::Relaxed) {
            0 => Ok(()),
            s => write!(
                f,
                "  {}",
                p.e(Tok::Subtle, format!("{:02}/{:02}", s >> 8, s & 0xff))
            ),
        }
    }

    /// `◇ Label · value`, the collapsed form of an answered prompt.
    fn collapsed(&self, f: &mut dyn fmt::Write, prompt: &str, value: &str) -> fmt::Result {
        let p = &self.cfg.pal;
        write!(
            f,
            "{} {} {} {}",
            p.e(Tok::Subtle, self.cfg.g.answered),
            p.e(Tok::Muted, prompt),
            p.e(Tok::Subtle, "·"),
            p.e(Tok::Fg, value)
        )
    }
}

impl Theme for UltravioletTheme {
    fn format_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.label(f, prompt, self.cfg.g.active, Tok::Accent)
    }

    fn format_error(&self, f: &mut dyn fmt::Write, err: &str) -> fmt::Result {
        let p = &self.cfg.pal;
        write!(
            f,
            "{} {}",
            p.e(Tok::Error, self.cfg.g.err),
            p.e(Tok::Fg, err)
        )
    }

    fn format_confirm_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<bool>,
    ) -> fmt::Result {
        self.format_prompt(f, prompt)?;
        let choices = match default {
            Some(true) => "[Y]es / [n]o",
            Some(false) => "[y]es / [N]o",
            None => "[Y]es / [N]o",
        };
        write!(f, "  {} ", self.cfg.pal.e(Tok::Subtle, choices))
    }

    fn format_confirm_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selection: Option<bool>,
    ) -> fmt::Result {
        match selection {
            Some(true) => self.collapsed(f, prompt, "yes"),
            Some(false) => self.collapsed(f, prompt, "no"),
            None => self.collapsed(f, prompt, &hint(&self.cfg.pal, "Skipped")),
        }
    }

    fn format_input_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<&str>,
    ) -> fmt::Result {
        let p = &self.cfg.pal;
        self.format_prompt(f, prompt)?;
        if let Some(default) = default {
            write!(f, " {}", p.e(Tok::Subtle, format!("({default})")))?;
        }
        // The input sits on its own line, indented to `col-2`.
        write!(f, "\n  {} ", p.e(Tok::Accent, self.cfg.g.caret))
    }

    fn format_input_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> fmt::Result {
        self.collapsed(f, prompt, sel)
    }

    fn format_password_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
    ) -> fmt::Result {
        self.collapsed(f, prompt, &"•".repeat(8))
    }

    fn format_multi_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        if selections.is_empty() {
            self.collapsed(f, prompt, &hint(&self.cfg.pal, "none"))
        } else {
            self.collapsed(f, prompt, &selections.join(", "))
        }
    }

    fn format_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        active: bool,
    ) -> fmt::Result {
        let p = &self.cfg.pal;
        if active {
            write!(
                f,
                "  {} {}",
                p.e(Tok::Accent, self.cfg.g.focus),
                p.e(Tok::Strong, text)
            )
        } else {
            write!(f, "    {}", p.e(Tok::Muted, text))
        }
    }

    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        let p = &self.cfg.pal;
        let g = &self.cfg.g;
        let cursor = if active {
            p.e(Tok::Accent, g.focus)
        } else {
            " ".repeat(g.focus.chars().count())
        };
        let (mark, mark_tok) = if checked {
            (g.checked, Tok::Accent)
        } else {
            (g.unchecked, Tok::Subtle)
        };
        let text_tok = if active { Tok::Strong } else { Tok::Muted };
        write!(
            f,
            "  {} {} {}",
            cursor,
            p.e(mark_tok, mark),
            p.e(text_tok, text)
        )
    }

    fn format_sort_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        picked: bool,
        active: bool,
    ) -> fmt::Result {
        let p = &self.cfg.pal;
        let g = &self.cfg.g;
        let cursor = if active {
            p.e(Tok::Accent, g.focus)
        } else {
            " ".repeat(g.focus.chars().count())
        };
        let (mark, mark_tok) = if picked {
            (g.chosen, Tok::Accent)
        } else {
            (g.pending, Tok::Subtle)
        };
        let text_tok = if active || picked {
            Tok::Strong
        } else {
            Tok::Muted
        };
        write!(
            f,
            "  {} {} {}",
            cursor,
            p.e(mark_tok, mark),
            p.e(text_tok, text)
        )
    }

    fn format_fuzzy_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        active: bool,
        highlight_matches: bool,
        matcher: &SkimMatcherV2,
        search_term: &str,
    ) -> fmt::Result {
        let p = &self.cfg.pal;
        if active {
            write!(f, "  {} ", p.e(Tok::Accent, self.cfg.g.focus))?;
        } else {
            write!(f, "    ")?;
        }
        let base = if active { Tok::Strong } else { Tok::Muted };
        let indices = if highlight_matches {
            matcher.fuzzy_indices(text, search_term).map(|(_, i)| i)
        } else {
            None
        };
        match indices {
            Some(indices) => {
                for (idx, c) in text.chars().enumerate() {
                    let style = p.style(if indices.contains(&idx) {
                        Tok::Accent
                    } else {
                        base
                    });
                    let style = if indices.contains(&idx) {
                        style.bold()
                    } else {
                        style
                    };
                    write!(f, "{}", style.for_stderr().apply_to(c))?;
                }
                Ok(())
            }
            None => write!(f, "{}", p.e(base, text)),
        }
    }

    fn format_fuzzy_select_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        search_term: &str,
        bytes_pos: usize,
    ) -> fmt::Result {
        let p = &self.cfg.pal;
        self.format_prompt(f, prompt)?;
        let (head, tail) = search_term.split_at(bytes_pos);
        write!(
            f,
            " {} {}{}{}",
            p.e(Tok::Accent, self.cfg.g.caret),
            p.e(Tok::Fg, head),
            p.e(Tok::Accent, self.cfg.g.cursor),
            p.e(Tok::Fg, tail)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut out = String::new();
        theme
            .format_input_prompt_selection(&mut out, "Name", "Ada")
            .unwrap();
        assert_eq!(out, "◇ Name · Ada");

        let theme = plain(config(true));
        let mut out = String::new();
        theme.format_prompt(&mut out, "Name").unwrap();
        assert_eq!(out, "* Name");
    }

    #[test]
    fn errors_carry_only_a_glyph() {
        let theme = plain(config(false));
        let mut out = String::new();
        theme.format_error(&mut out, "Enter a valid email").unwrap();
        assert_eq!(out, "✕ Enter a valid email");
    }
}
