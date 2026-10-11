//! Lull's two themes: "daylight" and "moonlight". Both are `ColorfulTheme` with its own
//! prompt and block styles dropped, so every part takes its colour from the palette, and
//! a few palette roles and glyphs changed. See docs/design/lull.md.
use paloma::{
    console::Style,
    terminal::ColorDepth,
    theme::{BlockTokens, ColorfulTheme, PromptTokens, Theme, Tokens},
};

/// `ColorfulTheme`'s tokens without its fixed prompt and block styles (a yellow mark, bold
/// questions and titles, green answers): those parts fall back to the palette.
fn base(t: Tokens) -> Tokens {
    t.prompts(|_| PromptTokens::default())
        .blocks(|_| BlockTokens::default())
}

/// 256-colour index `i256` (also on 24-bit terminals), or `basic` on 16 colours.
fn c(depth: ColorDepth, i256: u8, basic: Style) -> Style {
    match depth {
        ColorDepth::Basic => basic,
        _ => Style::new().color256(i256),
    }
}

pub fn day(d: ColorDepth) -> impl Theme + Send + Sync {
    let grey = Style::new().black().bright();
    ColorfulTheme.restyle(move |t| {
        base(t)
            .palette(|p| {
                p.subtle(c(d, 244, grey.clone()))
                    .faint(c(d, 244, grey))
                    .accent(c(d, 214, Style::new().yellow()))
                    .accent_strong(c(d, 208, Style::new().yellow()).bold())
                    .success(c(d, 71, Style::new().green()))
                    .info(c(d, 74, Style::new().cyan()))
                    .warning(c(d, 178, Style::new().yellow()))
                    .error(c(d, 167, Style::new().red()))
            })
            .glyphs(|g| g.prompt("☼").section("☼"))
    })
}

pub fn night(d: ColorDepth) -> impl Theme + Send + Sync {
    ColorfulTheme.restyle(move |t| {
        base(t)
            .palette(|p| {
                p.text(c(d, 146, Style::new()))
                    .muted(c(d, 103, Style::new()))
                    .subtle(c(d, 60, Style::new().black().bright()))
                    .faint(c(d, 238, Style::new().black().bright()))
                    .accent(c(d, 104, Style::new().blue()))
                    .accent_strong(c(d, 147, Style::new().blue().bright()))
                    .success(c(d, 108, Style::new().green()))
                    .info(c(d, 110, Style::new().cyan()))
                    .warning(c(d, 180, Style::new().yellow()))
                    .error(c(d, 174, Style::new().red().bright()))
            })
            .glyphs(|g| g.prompt("☾").answered("⋆").section("✧"))
    })
}

#[cfg(test)]
mod tests {
    use paloma::{console::measure_text_width, theme::GlyphName};

    use super::*;

    #[test]
    fn glyphs_are_one_column() {
        let (day, night) = (day(ColorDepth::Basic), night(ColorDepth::Basic));
        for theme in [day.tokens(), night.tokens()] {
            for name in [GlyphName::Prompt, GlyphName::Answered, GlyphName::Section] {
                let glyph = theme.get_glyphs().get(name);
                assert_eq!(measure_text_width(glyph), 1, "{glyph}");
            }
        }
    }
}
