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

use crate::theme::{
    BlockKind, Context, GlyphName, KeyValueView, MessageView, PanelView, Role, SectionView,
    TableView,
};

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

/// Pads `text` with spaces to `width` display columns.
fn pad(text: &str, width: usize) -> String {
    let w = console::measure_text_width(text);
    format!("{text}{}", " ".repeat(width.saturating_sub(w)))
}

/// A section title: the section glyph in `accent`, then the title.
pub fn format_section(f: &mut dyn fmt::Write, ctx: &Context, view: &SectionView) -> fmt::Result {
    let tokens = ctx.tokens();
    let look = tokens.get_blocks().resolve(BlockKind::Section, tokens);
    let glyph = tokens.get_glyphs().get(GlyphName::Section);
    if !glyph.is_empty() {
        let accent = tokens.get_palette().get(Role::accent);
        write!(f, "{} ", ctx.paint(accent, glyph))?;
    }
    write!(f, "{}", ctx.paint(&look.title_style, view.title))
}

/// Key/value rows: values aligned to the longest key plus the `gap` token.
pub fn format_key_value(f: &mut dyn fmt::Write, ctx: &Context, view: &KeyValueView) -> fmt::Result {
    let tokens = ctx.tokens();
    let look = tokens.get_blocks().resolve(BlockKind::KeyValue, tokens);
    let width = view
        .rows
        .iter()
        .map(|(k, _)| console::measure_text_width(k))
        .max()
        .unwrap_or(0)
        + look.gap;
    for (i, (key, value)) in view.rows.iter().enumerate() {
        if i > 0 {
            writeln!(f)?;
        }
        write!(
            f,
            "{}{}{}",
            ctx.paint(&look.key_style, key),
            " ".repeat(width - console::measure_text_width(key)),
            ctx.paint(&look.value_style, value)
        )?;
    }
    Ok(())
}

/// A table: columns padded to their widest cell, `gap` columns apart, the header in
/// `header_style`.
pub fn format_table(f: &mut dyn fmt::Write, ctx: &Context, view: &TableView) -> fmt::Result {
    let tokens = ctx.tokens();
    let look = tokens.get_blocks().resolve(BlockKind::Table, tokens);
    let columns = view
        .rows
        .iter()
        .map(Vec::len)
        .chain([view.header.len()])
        .max()
        .unwrap_or(0);
    let widths: Vec<usize> = (0..columns)
        .map(|i| {
            view.rows
                .iter()
                .chain([view.header.to_vec()].iter())
                .filter_map(|row| row.get(i))
                .map(|cell| console::measure_text_width(cell))
                .max()
                .unwrap_or(0)
        })
        .collect();
    let gap = " ".repeat(look.gap);
    let line = |cells: &[String], style: &console::Style| -> String {
        cells
            .iter()
            .zip(&widths)
            .map(|(cell, w)| ctx.paint(style, pad(cell, *w)).to_string())
            .collect::<Vec<_>>()
            .join(&gap)
    };
    let mut lines = Vec::new();
    if !view.header.is_empty() {
        lines.push(line(view.header, &look.header_style));
    }
    for row in view.rows {
        lines.push(line(row, &look.cell_style));
    }
    write!(f, "{}", lines.join("\n"))
}

/// A box: the title in the top border, one row per line with the status glyph before
/// the text and the trailing value right-aligned. The width is the terminal width minus
/// 2, clamped to the `min_width` / `max_width` tokens.
pub fn format_panel(f: &mut dyn fmt::Write, ctx: &Context, view: &PanelView) -> fmt::Result {
    let tokens = ctx.tokens();
    let look = tokens.get_blocks().resolve(BlockKind::Panel, tokens);
    let glyphs = tokens.get_glyphs();
    let g = |name| glyphs.get(name);
    let width = ctx
        .capabilities()
        .width()
        .saturating_sub(2)
        .clamp(look.min_width, look.max_width.max(look.min_width));
    let inner = width.saturating_sub(2);
    let edge = |s: &str| ctx.paint(&look.border_style, s.to_string()).to_string();
    let head_fill = inner.saturating_sub(console::measure_text_width(view.title) + 3);
    writeln!(
        f,
        "{}{} {} {}{}",
        edge(g(GlyphName::BoxTl)),
        edge(g(GlyphName::BoxH)),
        ctx.paint(&look.title_style, view.title),
        edge(&g(GlyphName::BoxH).repeat(head_fill)),
        edge(g(GlyphName::BoxTr))
    )?;
    for row in view.rows {
        let mut left = String::from(" ");
        if let Some(status) = row.get_status() {
            let msg = tokens.get_messages().resolve(status, tokens);
            if let Some(glyph) = msg.glyph {
                left += &format!("{} ", ctx.paint(&msg.glyph_style, glyph));
            }
        }
        left += &ctx.paint(&look.cell_style, row.text()).to_string();
        let right = row
            .get_trailing()
            .map(|t| ctx.paint(&look.trailing_style, t).to_string())
            .unwrap_or_default();
        let gap = inner.saturating_sub(
            console::measure_text_width(&left) + console::measure_text_width(&right) + 1,
        );
        writeln!(
            f,
            "{}{}{}{} {}",
            edge(g(GlyphName::BoxV)),
            left,
            " ".repeat(gap),
            right,
            edge(g(GlyphName::BoxV))
        )?;
    }
    write!(
        f,
        "{}{}{}",
        edge(g(GlyphName::BoxBl)),
        edge(&g(GlyphName::BoxH).repeat(inner)),
        edge(g(GlyphName::BoxBr))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{output::panel::PanelRow, terminal::Stream, theme::Tokens, Status};

    fn plain(draw: impl FnOnce(&mut String, &Context) -> fmt::Result) -> String {
        let tokens = Tokens::plain();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let mut out = String::new();
        draw(&mut out, &ctx).unwrap();
        out
    }

    #[test]
    fn key_values_align_to_the_longest_key() {
        let rows = [
            ("A".to_string(), "1".to_string()),
            ("Long key".into(), "2".into()),
        ];
        let out = plain(|f, ctx| format_key_value(f, ctx, &KeyValueView::new(&rows)));
        assert_eq!(out, "A          1\nLong key   2");
    }

    #[test]
    fn table_columns_use_display_width() {
        let header = ["Name".to_string(), "Mark".to_string()];
        let rows = vec![
            vec!["日本".to_string(), "✓".to_string()],
            vec!["Spain".to_string(), "x".to_string()],
        ];
        let out = plain(|f, ctx| format_table(f, ctx, &TableView::new(&header, &rows)));
        assert_eq!(out, "Name    Mark\n日本    ✓   \nSpain   x   ");
    }

    #[test]
    fn panel_rows_fill_the_box() {
        let rows = [
            PanelRow::new("Signed in")
                .status(Status::Success)
                .trailing("1.8s"),
            PanelRow::new("Plain row"),
        ];
        let out = plain(|f, ctx| format_panel(f, ctx, &PanelView::new("Done", &rows)));
        let widths: Vec<usize> = out.lines().map(console::measure_text_width).collect();
        assert!(widths.windows(2).all(|w| w[0] == w[1]), "{out}");
        assert!(out.starts_with("+- Done -"));
        assert!(out.contains("| Signed in "));
        assert!(out.contains(" 1.8s |"));
    }
}
