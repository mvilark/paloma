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

use console::Style;

use crate::theme::{
    Answer, AnsweredView, BlockKind, ConfirmButtonsView, ConfirmPromptView, Context,
    FlowSummaryView, FuzzySelectPromptView, GlyphName, InputPromptView, ItemView, KeyValueView,
    LegendView, LiveKind, MessageView, PanelView, PromptKind, PromptLook, PromptMessageView,
    PromptView, Role, SectionView, TableView, TaskState, TaskStepView, TaskView,
};

/// One message line: the indent for its depth, then the glyph, the word (followed by
/// `word_gap` spaces) and the text, each styled by the resolved [`MessageLook`](crate::theme::MessageLook).
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
        write!(
            f,
            "{}{}",
            ctx.paint(&look.word_style, word),
            " ".repeat(look.word_gap)
        )?;
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

/// A task line. Running: the spinner frame, the label and, if `show_elapsed`, the
/// elapsed time. Finished: the result drawn like a message of that status
/// ([`format_message`]) and the elapsed time.
pub fn format_task(f: &mut dyn fmt::Write, ctx: &Context, view: &TaskView) -> fmt::Result {
    let tokens = ctx.tokens();
    let look = tokens.get_live().resolve(LiveKind::Task, tokens);
    match view.state {
        TaskState::Running => {
            let frames = tokens.get_glyphs().get_spinner();
            if !frames.is_empty() {
                let glyph = frames[view.frame % frames.len()];
                write!(f, "{} ", ctx.paint(&look.spinner_style, glyph))?;
            }
            write!(f, "{}", ctx.paint(&look.label_style, view.text))?;
        }
        TaskState::Finished(status) => {
            format_message(f, ctx, &MessageView::new(status, view.text, 0))?;
        }
    }
    if look.show_elapsed {
        let secs = format!("{:.1}s", view.elapsed.as_secs_f64());
        write!(f, "  {}", ctx.paint(&look.elapsed_style, secs))?;
    }
    Ok(())
}

/// A step line under a task: the `step_indent` token as spaces, the nested glyph and
/// the text.
pub fn format_task_step(f: &mut dyn fmt::Write, ctx: &Context, view: &TaskStepView) -> fmt::Result {
    let tokens = ctx.tokens();
    let look = tokens.get_live().resolve(LiveKind::Task, tokens);
    write!(f, "{}", " ".repeat(look.step_indent))?;
    let glyph = tokens.get_glyphs().get(GlyphName::Nested);
    if !glyph.is_empty() {
        write!(f, "{} ", ctx.paint(&look.nested_style, glyph))?;
    }
    write!(f, "{}", ctx.paint(&look.step_style, view.text))
}

/// Writes the parts of a prompt line one space apart, skipping empty parts.
struct Parts<'f, 'c> {
    f: &'f mut dyn fmt::Write,
    ctx: &'c Context<'c>,
    wrote: bool,
}

impl<'f, 'c> Parts<'f, 'c> {
    fn new(f: &'f mut dyn fmt::Write, ctx: &'c Context<'c>) -> Self {
        Parts {
            f,
            ctx,
            wrote: false,
        }
    }

    /// `text` in `style`, after a space when something precedes it.
    fn part(&mut self, style: &Style, text: &str) -> fmt::Result {
        if text.is_empty() {
            return Ok(());
        }
        if self.wrote {
            write!(self.f, " ")?;
        }
        self.wrote = true;
        write!(self.f, "{}", self.ctx.paint(style, text))
    }

    /// A suffix or separator: `text` in `style`, after `gap` spaces when something
    /// precedes it.
    fn attached(&mut self, style: &Style, text: &str, gap: usize) -> fmt::Result {
        if text.is_empty() {
            return Ok(());
        }
        if self.wrote {
            write!(self.f, "{}", " ".repeat(gap))?;
        }
        self.wrote = true;
        write!(self.f, "{}", self.ctx.paint(style, text))
    }

    /// The question: the mark and the prompt, both only when the prompt is not empty.
    fn question(&mut self, look: &PromptLook, prompt: &str) -> fmt::Result {
        if !prompt.is_empty() {
            self.part(&look.mark_style, look.mark)?;
            self.part(&look.label_style, prompt)?;
        }
        Ok(())
    }

    /// The flow step counter (`02/05`) in `step_style`, after `step_gap` spaces, when
    /// the prompt is shown in a flow with its counter on.
    fn step(&mut self, look: &PromptLook, step: Option<(usize, usize)>) -> fmt::Result {
        match step {
            Some((n, total)) => self.attached(
                &look.step_style,
                &format!("{n:02}/{total:02}"),
                look.step_gap,
            ),
            None => Ok(()),
        }
    }

    /// Writes `value` after a space when something precedes it, even when the value is
    /// empty.
    fn value(&mut self, value: impl fmt::Display) -> fmt::Result {
        if self.wrote {
            write!(self.f, " ")?;
        }
        self.wrote = true;
        write!(self.f, "{value}")
    }
}

fn prompt_look(ctx: &Context, kind: PromptKind) -> PromptLook {
    let tokens = ctx.tokens();
    tokens.get_prompts().resolve(kind, tokens)
}

/// The suffix style: `invalid_suffix_style` while the row shows a validation error.
fn suffix_style(look: &PromptLook, invalid: bool) -> &Style {
    if invalid {
        &look.invalid_suffix_style
    } else {
        &look.suffix_style
    }
}

/// The question line of a list prompt: mark, question and suffix, then the paging
/// counter (` [Page 1/3] `) in `paging_style`.
pub fn format_prompt(f: &mut dyn fmt::Write, ctx: &Context, view: &PromptView) -> fmt::Result {
    let look = prompt_look(ctx, view.kind);
    let mut line = Parts::new(f, ctx);
    line.question(&look, view.prompt)?;
    line.step(&look, view.step)?;
    line.attached(suffix_style(&look, view.invalid), look.suffix, look.gap)?;
    if let Some((page, pages)) = view.paging {
        let counter = format!("[Page {page}/{pages}]");
        write!(f, " {} ", ctx.paint(&look.paging_style, counter))?;
    }
    Ok(())
}

/// The input line: mark, question, the default in `default_brackets` and `hint_style`,
/// the suffix (in `invalid_suffix_style` while the row shows an error), then a space
/// for the typed text.
pub fn format_input_prompt(
    f: &mut dyn fmt::Write,
    ctx: &Context,
    view: &InputPromptView,
) -> fmt::Result {
    let look = prompt_look(ctx, view.kind);
    let mut line = Parts::new(f, ctx);
    line.question(&look, view.prompt)?;
    line.step(&look, view.step)?;
    if let Some(default) = view.default {
        let (open, close) = look.default_brackets;
        line.part(&look.hint_style, &format!("{open}{default}{close}"))?;
    }
    line.attached(suffix_style(&look, view.invalid), look.suffix, look.gap)?;
    if line.wrote {
        write!(f, " ")?;
    }
    Ok(())
}

/// The confirm line: mark, question, the choices in `hint_style`, the suffix, then the
/// answer `Enter` gives in `default_style` (when `show_default`). With buttons, only
/// the mark, question and suffix.
pub fn format_confirm_prompt(
    f: &mut dyn fmt::Write,
    ctx: &Context,
    view: &ConfirmPromptView,
) -> fmt::Result {
    let look = prompt_look(ctx, PromptKind::Confirm);
    let mut line = Parts::new(f, ctx);
    line.question(&look, view.prompt)?;
    line.step(&look, view.step)?;
    if !view.buttons {
        let choices = match view.default {
            None => look.choices,
            Some(true) => look.choices_yes,
            Some(false) => look.choices_no,
        };
        line.part(&look.hint_style, choices)?;
    }
    line.attached(suffix_style(&look, view.invalid), look.suffix, look.gap)?;
    if let (false, true, Some(default)) = (view.buttons, look.show_default, view.default) {
        line.part(
            &look.default_style,
            if default { look.yes } else { look.no },
        )?;
    }
    Ok(())
}

/// The button row: the yes and the no button two spaces apart, the focused one in
/// `button_brackets` and `focused_button_style`, the other padded to the same width.
pub fn format_confirm_buttons(
    f: &mut dyn fmt::Write,
    ctx: &Context,
    view: &ConfirmButtonsView,
) -> fmt::Result {
    let look = prompt_look(ctx, PromptKind::Confirm);
    let (open, close) = look.button_brackets;
    let button = |label: &str, focused: bool| {
        if focused {
            ctx.paint(&look.focused_button_style, format!("{open}{label}{close}"))
        } else {
            let pad = |s: &str| " ".repeat(console::measure_text_width(s));
            ctx.paint(
                &look.button_style,
                format!("{}{label}{}", pad(open), pad(close)),
            )
        }
    };
    write!(
        f,
        "{}  {}",
        button(look.yes_button, view.yes_focused),
        button(look.no_button, !view.yes_focused)
    )
}

/// The search line: mark, question and suffix, a space, then the search in
/// `text_style` with the cursor glyph in `cursor_style` at the cursor. With an empty
/// cursor glyph the character under the cursor (a space at the end) is drawn in
/// `cursor_style` instead. An empty search with a placeholder shows the cursor, then
/// the placeholder in `placeholder_style`.
pub fn format_fuzzy_select_prompt(
    f: &mut dyn fmt::Write,
    ctx: &Context,
    view: &FuzzySelectPromptView,
) -> fmt::Result {
    let look = prompt_look(ctx, PromptKind::FuzzySelect);
    let mut line = Parts::new(f, ctx);
    line.question(&look, view.prompt)?;
    line.step(&look, view.step)?;
    line.attached(suffix_style(&look, view.invalid), look.suffix, look.gap)?;
    if line.wrote {
        write!(f, " ")?;
    }
    if let (true, Some(placeholder)) = (view.search.is_empty(), view.placeholder) {
        let cursor = if look.cursor.is_empty() {
            " "
        } else {
            look.cursor
        };
        return write!(
            f,
            "{}{}",
            ctx.paint(&look.cursor_style, cursor),
            ctx.paint(&look.placeholder_style, placeholder)
        );
    }
    let (head, tail) = view.search.split_at(view.cursor.min(view.search.len()));
    let text = |s: &str| ctx.paint(&look.text_style, s.to_string());
    if look.cursor.is_empty() {
        let mut chars = tail.chars();
        let under = chars.next().unwrap_or(' ');
        write!(
            f,
            "{}{}{}",
            text(head),
            ctx.paint(&look.cursor_style, under),
            text(chars.as_str())
        )
    } else {
        write!(
            f,
            "{}{}{}",
            text(head),
            ctx.paint(&look.cursor_style, look.cursor),
            text(tail)
        )
    }
}

/// One item: `item_indent` spaces, the focus glyph on the active item (spaces on the
/// others), the checked or unchecked glyph when the item has one, then the text in
/// `active_style` or `item_style` (`disabled_style` for a disabled item), with search
/// matches in `match_style` inside it, and the description, if any, after
/// `item_description_gap` spaces in `item_description_style`.
/// A sort item shows `checked` only while it is active (only the active item can be
/// picked up).
pub fn format_item(f: &mut dyn fmt::Write, ctx: &Context, view: &ItemView) -> fmt::Result {
    let look = prompt_look(ctx, view.kind);
    write!(f, "{}", " ".repeat(look.item_indent))?;
    if !look.focus.is_empty() {
        if view.active {
            write!(f, "{} ", ctx.paint(&look.focus_style, look.focus))?;
        } else {
            write!(
                f,
                "{} ",
                " ".repeat(console::measure_text_width(look.focus))
            )?;
        }
    }
    if let Some(checked) = view.checked {
        let on = checked && (view.kind != PromptKind::Sort || view.active);
        let (style, glyph) = if on {
            (&look.checked_style, look.checked)
        } else {
            (&look.unchecked_style, look.unchecked)
        };
        write!(f, "{} ", ctx.paint(style, glyph))?;
    }
    let style = if view.disabled {
        &look.disabled_style
    } else if view.active {
        &look.active_style
    } else {
        &look.item_style
    };
    match view.matches {
        Some(matches) => {
            for (i, c) in view.text.chars().enumerate() {
                if matches.contains(&i) {
                    write!(f, "{}", ctx.paint(style, ctx.paint(&look.match_style, c)))?;
                } else {
                    write!(f, "{}", ctx.paint(style, c))?;
                }
            }
        }
        None => write!(f, "{}", ctx.paint(style, view.text))?,
    }
    match view.description {
        Some(description) => write!(
            f,
            "{}{}",
            " ".repeat(look.item_description_gap),
            ctx.paint(&look.item_description_style, description)
        ),
        None => Ok(()),
    }
}

/// The line of keys under a prompt: `message_indent` spaces, then each `key action`
/// pair (the key in `legend_key_style`, the action in `legend_style`) joined by
/// `legend_separator` in `legend_style`.
pub fn format_legend(f: &mut dyn fmt::Write, ctx: &Context, view: &LegendView) -> fmt::Result {
    let look = prompt_look(ctx, view.kind);
    write!(f, "{}", " ".repeat(look.message_indent))?;
    for (i, (key, action)) in view.keys.iter().enumerate() {
        if i > 0 {
            write!(
                f,
                "{}",
                ctx.paint(&look.legend_style, look.legend_separator)
            )?;
        }
        write!(f, "{}", ctx.paint(&look.legend_key_style, key))?;
        if !key.is_empty() && !action.is_empty() {
            write!(f, " ")?;
        }
        write!(f, "{}", ctx.paint(&look.legend_style, action))?;
    }
    Ok(())
}

/// The `message_indent` token of the view's prompt, as spaces.
pub(crate) fn prompt_message_indent(
    f: &mut dyn fmt::Write,
    ctx: &Context,
    view: &PromptMessageView,
) -> fmt::Result {
    let indent = prompt_look(ctx, view.kind).message_indent;
    write!(f, "{}", " ".repeat(indent))
}

/// One line of the message row under a prompt: the `message_indent` token as spaces,
/// then the line as [`format_message`] draws it (a nested message at its depth).
pub fn format_prompt_message(
    f: &mut dyn fmt::Write,
    ctx: &Context,
    view: &PromptMessageView,
) -> fmt::Result {
    prompt_message_indent(f, ctx, view)?;
    format_message(
        f,
        ctx,
        &MessageView::new(view.status, view.text, view.depth),
    )
}

/// The summary line a finished flow leaves with `Done::Collapse`, drawn like an
/// answered line with the `input` prompt look (which falls back to `base`): the
/// answered mark, the title in `answered_label_style`, the separator, then
/// `{steps} {flow_steps_word}` in `value_style`.
pub fn format_flow_summary(
    f: &mut dyn fmt::Write,
    ctx: &Context,
    view: &FlowSummaryView,
) -> fmt::Result {
    let look = prompt_look(ctx, PromptKind::Input);
    let mut line = Parts::new(f, ctx);
    if !view.title.is_empty() {
        line.part(&look.answered_mark_style, look.answered_mark)?;
        line.part(&look.answered_label_style, view.title)?;
    }
    line.attached(&look.separator_style, look.separator, look.gap)?;
    let steps = format!("{} {}", view.steps, look.flow_steps_word);
    line.value(ctx.paint(&look.value_style, steps))
}

/// The line an answered prompt leaves: answered mark, question, separator, then the
/// answer in `value_style` (a list joined by `, ` in `list_separator_style`, a password as `hidden_value` or mask
/// glyphs), or `empty_value` in `empty_style` for a skipped answer or an empty list.
/// A cancelled prompt shows `cancelled_value` in `cancelled_style`, when it is set.
pub fn format_answered(f: &mut dyn fmt::Write, ctx: &Context, view: &AnsweredView) -> fmt::Result {
    let look = prompt_look(ctx, view.kind);
    let mut line = Parts::new(f, ctx);
    if !view.prompt.is_empty() {
        line.part(&look.answered_mark_style, look.answered_mark)?;
        line.part(&look.answered_label_style, view.prompt)?;
    }
    line.attached(&look.separator_style, look.separator, look.gap)?;
    let value = |text: &str| ctx.paint(&look.value_style, text.to_string()).to_string();
    let empty = look
        .empty_value
        .map(|e| ctx.paint(&look.empty_style, e).to_string());
    match view.answer {
        Answer::Text(text) => line.value(value(text)),
        Answer::Bool(yes) => line.value(value(if yes { look.yes } else { look.no })),
        Answer::List([]) if empty.is_some() => line.value(empty.unwrap_or_default()),
        Answer::List(items) => {
            let items: Vec<String> = items.iter().map(|item| value(item)).collect();
            let separator = ctx.paint(&look.list_separator_style, ", ").to_string();
            line.value(items.join(&separator))
        }
        Answer::Hidden(len) => {
            let hidden = look
                .hidden_value
                .map_or_else(|| look.mask.repeat(len), str::to_string);
            line.value(value(&hidden))
        }
        Answer::Skipped => match empty {
            Some(empty) => line.value(empty),
            None => Ok(()),
        },
        Answer::Cancelled => match look.cancelled_value {
            Some(cancelled) => line.value(ctx.paint(&look.cancelled_style, cancelled)),
            None => Ok(()),
        },
    }
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

    fn plain_with(
        f: impl FnOnce(crate::theme::PromptTokens) -> crate::theme::PromptTokens,
        draw: impl FnOnce(&mut String, &Context) -> fmt::Result,
    ) -> String {
        let tokens = Tokens::plain().prompts(f);
        let ctx = Context::new(&tokens, Stream::Stderr);
        let mut out = String::new();
        draw(&mut out, &ctx).unwrap();
        out
    }

    #[test]
    fn items_keep_columns_for_indent_and_wide_focus() {
        let item = |active, checked| {
            plain_with(
                |p| p.base(|b| b.item_indent(2).focus("→→")),
                |f, ctx| {
                    let view = ItemView::new(
                        PromptKind::MultiSelect,
                        "b",
                        active,
                        checked,
                        None,
                        None,
                        false,
                    );
                    format_item(f, ctx, &view)
                },
            )
        };
        assert_eq!(item(true, Some(true)), "  →→ [x] b");
        assert_eq!(item(false, Some(false)), "     [ ] b");
        assert_eq!(item(false, None), "     b");
    }

    #[test]
    fn sort_items_show_the_pick_only_on_the_active_item() {
        let item = |active| {
            plain(|f, ctx| {
                let view =
                    ItemView::new(PromptKind::Sort, "b", active, Some(true), None, None, false);
                format_item(f, ctx, &view)
            })
        };
        assert_eq!(item(true), "> [x] b");
        assert_eq!(item(false), "  [ ] b");
    }

    #[test]
    fn empty_answers_show_the_empty_value_when_set() {
        let answered = |kind, answer| {
            plain_with(
                |p| p.base(|b| b.empty_value("none")),
                |f, ctx| format_answered(f, ctx, &AnsweredView::new(kind, "Pick", answer)),
            )
        };
        assert_eq!(
            answered(PromptKind::MultiSelect, Answer::List(&[])),
            "Pick: none"
        );
        assert_eq!(answered(PromptKind::Confirm, Answer::Skipped), "Pick none");
        assert_eq!(
            answered(PromptKind::MultiSelect, Answer::List(&["a", "b"])),
            "Pick: a, b"
        );
        let unset = plain(|f, ctx| {
            let view = AnsweredView::new(PromptKind::MultiSelect, "Pick", Answer::List(&[]));
            format_answered(f, ctx, &view)
        });
        assert_eq!(unset, "Pick: ");
    }

    #[test]
    fn buttons_pad_the_unfocused_one_to_the_brackets() {
        let out = plain_with(
            |p| p.base(|b| b.button_brackets(("<<", ">"))),
            |f, ctx| format_confirm_buttons(f, ctx, &ConfirmButtonsView::new(false)),
        );
        assert_eq!(out, "  Yes   <<No>");
    }

    #[test]
    fn an_empty_cursor_glyph_marks_the_character_under_the_cursor() {
        let search = |cursor| {
            plain_with(
                |p| p.fuzzy_select(|s| s.cursor("")),
                |f, ctx| {
                    let view = FuzzySelectPromptView::new("Find", "ab", cursor, None, false, None);
                    format_fuzzy_select_prompt(f, ctx, &view)
                },
            )
        };
        // Plain tokens draw no style, so the character stays where it was.
        assert_eq!(search(0), "Find ab");
        assert_eq!(search(2), "Find ab ");
    }

    #[test]
    fn an_empty_search_shows_the_cursor_then_the_placeholder() {
        let search = |cursor: &'static str| {
            plain_with(
                |p| p.fuzzy_select(|s| s.cursor(cursor)),
                |f, ctx| {
                    let view =
                        FuzzySelectPromptView::new("Find", "", 0, Some("fruit"), false, None);
                    format_fuzzy_select_prompt(f, ctx, &view)
                },
            )
        };
        assert_eq!(search("|"), "Find |fruit");
        assert_eq!(search(""), "Find  fruit");
    }

    #[test]
    fn the_word_is_followed_by_word_gap_spaces() {
        let hint = |gap: Option<usize>| {
            let tokens = Tokens::plain().messages(|m| match gap {
                Some(gap) => m.hint(|h| h.word_gap(gap)),
                None => m,
            });
            let ctx = Context::new(&tokens, Stream::Stderr);
            let mut out = String::new();
            format_message(&mut out, &ctx, &MessageView::new(Status::Hint, "try", 0)).unwrap();
            out
        };
        assert_eq!(hint(None), "hint: try");
        assert_eq!(hint(Some(2)), "hint:  try");
        assert_eq!(hint(Some(0)), "hint:try");
    }

    #[test]
    fn prompt_messages_are_indented_messages() {
        use crate::theme::MessageSource;
        let line = |depth| {
            plain_with(
                |p| p.input(|i| i.message_indent(2)),
                |f, ctx| {
                    let view = PromptMessageView::new(
                        PromptKind::Input,
                        Status::Error,
                        "Bad",
                        depth,
                        MessageSource::Validation,
                    );
                    format_prompt_message(f, ctx, &view)
                },
            )
        };
        assert_eq!(line(0), "  error: Bad");
        assert_eq!(line(1), "    error: Bad");
    }

    #[test]
    fn items_show_their_description_after_the_gap() {
        let item = |gap: Option<usize>, description| {
            plain_with(
                |p| match gap {
                    Some(gap) => p.base(|b| b.item_description_gap(gap)),
                    None => p,
                },
                |f, ctx| {
                    let view = ItemView::new(
                        PromptKind::Select,
                        "b",
                        false,
                        None,
                        None,
                        description,
                        false,
                    );
                    format_item(f, ctx, &view)
                },
            )
        };
        assert_eq!(item(None, Some("more")), "  b  more");
        assert_eq!(item(Some(4), Some("more")), "  b    more");
        assert_eq!(item(None, None), "  b");
    }

    #[test]
    fn disabled_items_use_the_disabled_style_with_matches_inside() {
        let styled = |disabled, matches: Option<&[usize]>| {
            let tokens = Tokens::plain().prompts(|p| {
                p.base(|b| {
                    b.disabled_style(Style::new().red().force_styling(true))
                        .match_style(Style::new().bold().force_styling(true))
                })
            });
            let ctx = Context::new(&tokens, Stream::Stderr);
            let mut out = String::new();
            let view = ItemView::new(
                PromptKind::FuzzySelect,
                "ab",
                true,
                None,
                matches,
                None,
                disabled,
            );
            format_item(&mut out, &ctx, &view).unwrap();
            out
        };
        // Plain text for an enabled item; the disabled style, with the matched
        // character nested in it, for a disabled one.
        assert_eq!(styled(false, None), "> ab");
        assert_eq!(styled(true, None), "> \x1b[31mab\x1b[0m");
        assert_eq!(
            styled(true, Some(&[1])),
            "> \x1b[31ma\x1b[0m\x1b[31m\x1b[1mb\x1b[0m\x1b[0m"
        );
    }

    #[test]
    fn the_legend_lists_keys_and_actions() {
        let legend =
            |keys: &[(&str, &str)],
             tokens: fn(crate::theme::PromptTokens) -> crate::theme::PromptTokens| {
                plain_with(tokens, |f, ctx| {
                    format_legend(f, ctx, &LegendView::new(PromptKind::Select, keys))
                })
            };
        let keys = [("up/down", "move"), ("enter", "choose")];
        assert_eq!(legend(&keys, |p| p), "up/down move  enter choose");
        assert_eq!(legend(&[], |p| p), "");
        assert_eq!(
            legend(&keys, |p| p
                .base(|b| b.legend_separator(" | ").message_indent(2))),
            "  up/down move | enter choose"
        );
    }

    #[test]
    fn the_legend_defaults_per_prompt_and_plain_uses_ascii_arrows() {
        let legend = |tokens: &Tokens, kind| {
            let look = tokens.get_prompts().resolve(kind, tokens);
            look.legend
                .iter()
                .map(|(k, a)| format!("{k} {a}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let colorful = Tokens::colorful();
        assert_eq!(
            legend(&colorful, PromptKind::Select),
            "↑↓ move, enter choose"
        );
        assert_eq!(legend(&colorful, PromptKind::Input), "enter submit");
        assert_eq!(legend(&colorful, PromptKind::Password), "enter submit");
        assert_eq!(legend(&colorful, PromptKind::Confirm), "");
        assert_eq!(
            legend(&colorful, PromptKind::MultiSelect),
            "↑↓ move, space check, a all, enter submit"
        );
        assert_eq!(
            legend(&colorful, PromptKind::Sort),
            "↑↓ move, space pick up, enter submit"
        );
        assert_eq!(
            legend(&colorful, PromptKind::FuzzySelect),
            "↑↓ move, enter choose"
        );
        let plain = Tokens::plain();
        assert_eq!(
            legend(&plain, PromptKind::Select),
            "up/down move, enter choose"
        );
        assert_eq!(
            legend(&plain, PromptKind::Sort),
            "up/down move, space pick up, enter submit"
        );
        assert_eq!(legend(&plain, PromptKind::Input), "enter submit");
        let look = plain.get_prompts().resolve(PromptKind::Select, &plain);
        assert_eq!(look.legend_back, ("esc", "back"));
        assert_eq!(look.legend_separator, "  ");
    }

    #[test]
    fn a_cancelled_answer_shows_the_cancelled_value_only_when_set() {
        let answered = |value: bool| {
            plain_with(
                |p| {
                    if value {
                        p.base(|b| b.cancelled_value("cancelled"))
                    } else {
                        p
                    }
                },
                |f, ctx| {
                    let view = AnsweredView::new(PromptKind::Input, "Name", Answer::Cancelled);
                    format_answered(f, ctx, &view)
                },
            )
        };
        assert_eq!(answered(false), "Name:");
        assert_eq!(answered(true), "Name: cancelled");
    }
}
