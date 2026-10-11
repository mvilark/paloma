use std::{fmt, io, sync::Arc};

use console::{measure_text_width, Term, TermTarget};

#[cfg(feature = "fuzzy-select")]
use crate::theme::FuzzySelectPromptView;
use crate::{
    options::Done,
    output::message::Message,
    terminal::Stream,
    theme::{
        Answer, AnsweredView, ConfirmButtonsView, ConfirmPromptView, Context, InputPromptView,
        ItemView, LegendView, MessageSource, PromptKind, PromptLook, PromptMessageView, PromptView,
        Theme, Tokens,
    },
    Error, Result,
};

/// The theme a renderer uses: the prompt's own, or the app-wide one.
enum ThemeHandle<'a> {
    Prompt(&'a dyn Theme),
    App(Arc<dyn Theme + Send + Sync>),
}

impl<'a> std::ops::Deref for ThemeHandle<'a> {
    type Target = dyn Theme + 'a;

    fn deref(&self) -> &(dyn Theme + 'a) {
        match self {
            ThemeHandle::Prompt(theme) => *theme,
            ThemeHandle::App(theme) => &**theme,
        }
    }
}

/// How many screen rows `text` takes once written as whole lines on a terminal `width`
/// columns wide, counting the rows of lines that wrap.
fn rows(text: &str, width: usize) -> usize {
    text.split('\n').map(|line| line_rows(line, width)).sum()
}

/// How many screen rows one line takes on a terminal `width` columns wide.
fn line_rows(line: &str, width: usize) -> usize {
    measure_text_width(line).max(1).div_ceil(width.max(1))
}

/// Where the terminal cursor goes for column `column` (in display columns) of line
/// `line` of a frame made of `lines`, on a terminal `width` columns wide: the screen
/// row (counted from the frame's first row, with the rows of wrapped lines) and
/// column. `extra_row` is true when that row is past the rows the line itself takes:
/// the cursor sits right after a line that exactly fills the width, so the frame
/// needs one more row for it, right after that line.
fn cursor_position(
    lines: &[&str],
    line: usize,
    column: usize,
    width: usize,
) -> (usize, usize, bool) {
    let width = width.max(1);
    let before: usize = lines[..line].iter().map(|l| line_rows(l, width)).sum();
    let extra_row = column / width >= line_rows(lines[line], width);
    (before + column / width, column % width, extra_row)
}

/// What an input or password prompt shows where the typed text goes.
#[cfg_attr(not(feature = "password"), allow(dead_code))]
pub(crate) enum Shown<'s> {
    /// The typed text, split at the cursor.
    Text { before: &'s str, after: &'s str },
    /// The input is empty: the placeholder, with the cursor before it.
    Placeholder(&'s str),
    /// A masked password: one glyph per typed char, split at the cursor.
    Mask {
        before: usize,
        after: usize,
        glyph: char,
    },
    /// A hidden password: nothing.
    Hidden,
}

/// The message row of a prompt, if it shows anything, and where it comes from.
pub(crate) type RowMessage<'m> = Option<(&'m Message<'static>, MessageSource)>;

/// How many lines the row takes: the message and every nested message.
pub(crate) fn row_lines(row: RowMessage) -> usize {
    fn count(message: &Message) -> usize {
        1 + message.children().iter().map(count).sum::<usize>()
    }
    row.map_or(0, |(message, _)| count(message))
}

/// Draws prompts through a theme, as frames.
///
/// A prompt draws its whole state as one frame (question, items, typed text, message
/// row, legend) on every key, and the new frame replaces the previous one on screen. A
/// frame can be committed (it stays on screen, still part of the prompt, and the next
/// frame goes under it), the whole prompt can be erased, and the live frame can be
/// kept (left on screen without its row and legend, ending the prompt).
pub(crate) struct TermThemeRenderer<'a> {
    term: &'a Term,
    theme: ThemeHandle<'a>,
    /// The theme's tokens, read once when the prompt is shown.
    tokens: Tokens,
    /// The stream `term` writes to, so styles follow its terminal.
    stream: Stream,
    /// Rows of committed frames above the live one, still part of the prompt.
    kept: usize,
    /// Rows of the live frame.
    rows: usize,
    /// The row of the live frame the terminal cursor is on.
    cursor_row: usize,
    /// The live frame without its message row and legend: what `keep` leaves.
    body: String,
    /// The same for each committed frame.
    committed: Vec<String>,
    /// Whether frames end with the legend line.
    legend: bool,
    /// Whether the legend lists the key that goes back (`Esc`).
    back: bool,
    /// Pauses a running live component (a task's spinner) while the prompt is shown.
    _pause: crate::live::engine::Pause,
}

impl<'a> TermThemeRenderer<'a> {
    pub fn new(term: &'a Term, theme: Option<&'a dyn Theme>) -> TermThemeRenderer<'a> {
        let theme = match theme {
            Some(theme) => ThemeHandle::Prompt(theme),
            None => ThemeHandle::App(super::app::current()),
        };
        TermThemeRenderer {
            term,
            tokens: theme.tokens().into_owned(),
            stream: match term.target() {
                TermTarget::Stdout => Stream::Stdout,
                _ => Stream::Stderr,
            },
            theme,
            kept: 0,
            rows: 0,
            cursor_row: 0,
            body: String::new(),
            committed: Vec::new(),
            legend: false,
            back: false,
            _pause: crate::live::engine::Pause::new(),
        }
    }

    /// The context hooks draw with: this renderer's tokens and the term's stream.
    fn ctx(&self) -> Context<'_> {
        Context::new(&self.tokens, self.stream)
    }

    /// The resolved look of one prompt.
    pub fn look(&self, kind: PromptKind) -> PromptLook {
        self.tokens.get_prompts().resolve(kind, &self.tokens)
    }

    /// Replaces the live frame with `text` (the whole frame, without a final newline)
    /// and puts the terminal cursor at `cursor`, a line index in `text` and a column
    /// in display columns, or leaves it after the last line when `None`.
    fn frame(&mut self, text: &str, cursor: Option<(usize, usize)>) -> Result {
        let width = self.term.size().1 as usize;
        if self.rows > 0 {
            self.term.move_cursor_up(self.cursor_row)?;
            // `\r`, then clear to the end of the screen.
            self.term.clear_to_end_of_screen()?;
        }
        let lines: Vec<&str> = text.split('\n').collect();
        let mut rows = rows(text, width);
        let target = cursor.map(|(line, column)| {
            let line = line.min(lines.len() - 1);
            (line, cursor_position(&lines, line, column, width))
        });
        let mut out = String::with_capacity(text.len() + 2);
        match target {
            // The cursor needs a row of its own after its line.
            Some((line, (_, _, true))) => {
                rows += 1;
                for (i, l) in lines.iter().enumerate() {
                    if i > 0 {
                        out.push('\n');
                    }
                    out.push_str(l);
                    if i == line {
                        out.push_str(if i + 1 == lines.len() { "\r\n" } else { "\n" });
                    }
                }
            }
            _ => out.push_str(text),
        }
        self.term.write_str(&out)?;
        let last = rows - 1;
        self.cursor_row = last;
        if let Some((_, (row, column, _))) = target {
            self.term.move_cursor_up(last - row)?;
            self.term.write_str("\r")?;
            self.term.move_cursor_right(column)?;
            self.cursor_row = row;
        }
        self.rows = rows;
        self.term.flush()?;
        Ok(())
    }

    /// Keeps the live frame on screen as part of the prompt: the next frame is drawn
    /// under it, and [`erase`](Self::erase) still removes it.
    #[cfg_attr(not(feature = "password"), allow(dead_code))]
    pub fn commit(&mut self) -> Result {
        if self.rows > 0 {
            self.term
                .move_cursor_down(self.rows - 1 - self.cursor_row)?;
            self.term.write_str("\r\n")?;
            self.committed.push(std::mem::take(&mut self.body));
        }
        self.kept += self.rows;
        self.rows = 0;
        self.cursor_row = 0;
        Ok(())
    }

    /// Erases the whole prompt: the live frame and the committed ones above it. The
    /// cursor ends where the prompt started.
    pub fn erase(&mut self) -> Result {
        self.term.move_cursor_up(self.cursor_row + self.kept)?;
        self.term.clear_to_end_of_screen()?;
        self.kept = 0;
        self.rows = 0;
        self.cursor_row = 0;
        self.body.clear();
        self.committed.clear();
        Ok(())
    }

    /// Leaves the prompt on screen as it was when answered, without its message row
    /// and legend: the committed frames and the live one are redrawn from their
    /// bodies, and the cursor ends on the line below them.
    pub fn keep(&mut self) -> Result {
        let mut parts = std::mem::take(&mut self.committed);
        if self.rows > 0 {
            parts.push(std::mem::take(&mut self.body));
        }
        self.erase()?;
        if !parts.is_empty() {
            self.term.write_str(&parts.join("\n"))?;
            self.term.write_str("\r\n")?;
        }
        Ok(())
    }

    /// Ends a prompt that was answered: per `done`, the answered line replaces the
    /// prompt (`Collapse`), the prompt stays (`Keep`) or nothing stays (`Erase`). The
    /// cursor is shown again.
    pub fn finish(&mut self, done: Done, kind: PromptKind, prompt: &str, answer: Answer) -> Result {
        match done {
            Done::Collapse => {
                self.erase()?;
                self.answered(kind, prompt, answer)?;
            }
            Done::Keep => self.keep()?,
            Done::Erase => self.erase()?,
        }
        self.term.show_cursor()?;
        self.term.flush()?;
        Ok(())
    }

    /// Draws the line a preset or unattended answer leaves, for a prompt that was never
    /// shown: the answered line unless `done` is `Erase` (`Keep` draws like
    /// `Collapse`, since nothing was shown).
    pub fn finish_preset(
        &mut self,
        done: Done,
        kind: PromptKind,
        prompt: &str,
        answer: Answer,
    ) -> Result {
        match done {
            Done::Collapse | Done::Keep => self.answered(kind, prompt, answer),
            Done::Erase => Ok(()),
        }
    }

    /// Ends a prompt the user left with `Esc`: nothing stays, whatever `done` says.
    pub fn back(&mut self) -> Result {
        self.erase()?;
        self.term.show_cursor()?;
        self.term.flush()?;
        Ok(())
    }

    /// Ends a prompt the user cancelled with `Ctrl+C`: the prompt is erased, the
    /// cancelled line is drawn when the theme has a `cancelled_value` (and the prompt a
    /// question), the cursor is shown, and the result is [`Error::Cancelled`].
    pub fn cancel<T>(&mut self, kind: PromptKind, prompt: Option<&str>) -> Result<T> {
        self.erase()?;
        if let (Some(prompt), Some(_)) = (prompt, self.look(kind).cancelled_value) {
            self.answered(kind, prompt, Answer::Cancelled)?;
        }
        self.term.show_cursor()?;
        self.term.flush()?;
        Err(Error::Cancelled)
    }

    /// Sets whether frames end with the legend line, and whether it lists the key that
    /// goes back.
    pub fn set_legend(&mut self, enabled: bool, back: bool) {
        self.legend = enabled;
        self.back = back;
    }

    /// How many rows the legend of a `kind` prompt takes (0 or 1).
    pub fn legend_lines(&self, kind: PromptKind) -> usize {
        usize::from(self.legend && !self.look(kind).legend.is_empty())
    }

    /// Appends the lines of the message row to a frame: the message, then its nested
    /// messages one level deeper each, like [`Message`] prints them.
    fn push_row(&self, lines: &mut Vec<String>, kind: PromptKind, row: RowMessage) -> fmt::Result {
        fn push(
            this: &TermThemeRenderer,
            ctx: &Context,
            lines: &mut Vec<String>,
            kind: PromptKind,
            message: &Message,
            source: MessageSource,
            depth: usize,
        ) -> fmt::Result {
            let mut line = String::new();
            let view =
                PromptMessageView::new(kind, message.status(), message.text(), depth, source);
            this.theme.format_prompt_message(&mut line, ctx, &view)?;
            lines.push(line);
            for child in message.children() {
                push(this, ctx, lines, kind, child, source, depth + 1)?;
            }
            Ok(())
        }
        if let Some((message, source)) = row {
            push(self, &self.ctx(), lines, kind, message, source, 0)?;
        }
        Ok(())
    }

    /// Appends the legend line to a frame, when it is on and the prompt has keys.
    fn push_legend(&self, lines: &mut Vec<String>, kind: PromptKind) -> fmt::Result {
        if self.legend_lines(kind) == 0 {
            return Ok(());
        }
        let look = self.look(kind);
        let mut keys: Vec<(&str, &str)> = look.legend.to_vec();
        if self.back {
            keys.push(look.legend_back);
        }
        let mut line = String::new();
        let view = LegendView::new(kind, &keys);
        self.theme.format_legend(&mut line, &self.ctx(), &view)?;
        lines.push(line);
        Ok(())
    }

    /// Adds the message row and the legend after the lines of a frame's body. Returns
    /// the body as text (what `keep` leaves) and the whole frame.
    fn tail(
        &self,
        mut lines: Vec<String>,
        kind: PromptKind,
        row: RowMessage,
    ) -> std::result::Result<(String, String), fmt::Error> {
        let body = lines.join("\n");
        self.push_row(&mut lines, kind, row)?;
        self.push_legend(&mut lines, kind)?;
        Ok((body, lines.join("\n")))
    }

    /// The frame of a list prompt (select, multi select, sort): the question line
    /// with its paging counter (when there is a question), the items, then the row.
    pub fn prompt_frame<'i>(
        &mut self,
        kind: PromptKind,
        prompt: Option<&str>,
        paging: Option<(usize, usize)>,
        invalid: bool,
        items: impl Iterator<Item = ItemView<'i>>,
        row: RowMessage,
    ) -> Result {
        let (body, text) = (|| -> std::result::Result<(String, String), fmt::Error> {
            let ctx = self.ctx();
            let mut lines = Vec::new();
            if let Some(prompt) = prompt {
                let mut line = String::new();
                let view = PromptView::new(kind, prompt, paging, invalid);
                self.theme.format_prompt(&mut line, &ctx, &view)?;
                lines.push(line);
            }
            for item in items {
                let mut line = String::new();
                self.theme.format_item(&mut line, &ctx, &item)?;
                lines.push(line);
            }
            self.tail(lines, kind, row)
        })()
        .map_err(io::Error::other)?;
        self.body = body;
        self.frame(&text, None)
    }

    /// The frame of an input or password prompt: the input line with the typed text
    /// (or what stands for it), then the row. The terminal cursor goes where the next
    /// typed char will be.
    pub fn input_frame(
        &mut self,
        kind: PromptKind,
        prompt: &str,
        default: Option<&str>,
        invalid: bool,
        shown: Shown,
        row: RowMessage,
    ) -> Result {
        let (body, text, cursor) = (|| -> std::result::Result<_, fmt::Error> {
            let ctx = self.ctx();
            let look = self.look(kind);
            let mut head = String::new();
            let view = InputPromptView::new(kind, prompt, default, invalid);
            self.theme.format_input_prompt(&mut head, &ctx, &view)?;
            let line = head.matches('\n').count();
            let start = measure_text_width(head.rsplit('\n').next().unwrap_or(""));
            let mut paint = |style: &console::Style, text: &str| {
                if !text.is_empty() {
                    head.push_str(&ctx.paint(style, text).to_string());
                }
            };
            let column = match shown {
                Shown::Text { before, after } => {
                    paint(&look.text_style, &format!("{before}{after}"));
                    start + measure_text_width(before)
                }
                Shown::Placeholder(placeholder) => {
                    paint(&look.placeholder_style, placeholder);
                    start
                }
                Shown::Mask {
                    before,
                    after,
                    glyph,
                } => {
                    let glyph = glyph.to_string();
                    paint(&look.text_style, &glyph.repeat(before + after));
                    start + before * measure_text_width(&glyph)
                }
                Shown::Hidden => start,
            };
            let (body, text) = self.tail(vec![head], kind, row)?;
            Ok((body, text, (line, column)))
        })()
        .map_err(io::Error::other)?;
        self.body = body;
        self.frame(&text, Some(cursor))
    }

    /// The frame of a confirm prompt: its line, or its question with the buttons on
    /// the line under it (`buttons` says whether `Yes` has the focus), then the row.
    pub fn confirm_frame(
        &mut self,
        prompt: &str,
        default: Option<bool>,
        invalid: bool,
        buttons: Option<bool>,
        row: RowMessage,
    ) -> Result {
        let (body, text) = (|| -> std::result::Result<(String, String), fmt::Error> {
            let ctx = self.ctx();
            let mut line = String::new();
            let view = ConfirmPromptView::new(prompt, default, buttons.is_some(), invalid);
            self.theme.format_confirm_prompt(&mut line, &ctx, &view)?;
            if let Some(yes_focused) = buttons {
                line.push('\n');
                let view = ConfirmButtonsView::new(yes_focused);
                self.theme.format_confirm_buttons(&mut line, &ctx, &view)?;
            }
            self.tail(vec![line], PromptKind::Confirm, row)
        })()
        .map_err(io::Error::other)?;
        self.body = body;
        self.frame(&text, None)
    }

    /// The frame of a fuzzy select: the search line, the matching items, then the row.
    /// `cursor` is a byte offset into `search`.
    #[cfg(feature = "fuzzy-select")]
    #[allow(clippy::too_many_arguments)]
    pub fn fuzzy_frame<'i>(
        &mut self,
        prompt: &str,
        search: &str,
        cursor: usize,
        placeholder: Option<&str>,
        invalid: bool,
        items: impl Iterator<Item = ItemView<'i>>,
        row: RowMessage,
    ) -> Result {
        let (body, text) = (|| -> std::result::Result<(String, String), fmt::Error> {
            let ctx = self.ctx();
            let mut line = String::new();
            let placeholder = placeholder.filter(|_| search.is_empty());
            let view = FuzzySelectPromptView::new(prompt, search, cursor, placeholder, invalid);
            self.theme
                .format_fuzzy_select_prompt(&mut line, &ctx, &view)?;
            let mut lines = vec![line];
            for item in items {
                let mut line = String::new();
                self.theme.format_item(&mut line, &ctx, &item)?;
                lines.push(line);
            }
            self.tail(lines, PromptKind::FuzzySelect, row)
        })()
        .map_err(io::Error::other)?;
        self.body = body;
        self.frame(&text, None)
    }

    /// The line an answered prompt leaves, written as a line of its own where the
    /// cursor is (after [`erase`](Self::erase), where the prompt was).
    pub fn answered(&mut self, kind: PromptKind, prompt: &str, answer: Answer) -> Result {
        let mut line = String::new();
        let view = AnsweredView::new(kind, prompt, answer);
        self.theme
            .format_answered(&mut line, &self.ctx(), &view)
            .map_err(io::Error::other)?;
        Ok(self.term.write_line(&line)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ColorfulTheme;

    #[test]
    fn rows_count_display_width_and_wrapping() {
        assert_eq!(rows("", 10), 1);
        assert_eq!(rows("a\nb", 10), 2);
        assert_eq!(rows(&"ñ".repeat(10), 10), 1); // 20 bytes, 10 columns
        assert_eq!(rows(&"a".repeat(11), 10), 2);
        assert_eq!(rows("\x1b[31mred\x1b[0m", 3), 1); // escape codes take no room
    }

    #[test]
    fn the_cursor_follows_wrapped_lines() {
        let lines = ["0123456789abc", "x"];
        // Line 0 takes rows 0 and 1 on a 10-column terminal.
        assert_eq!(cursor_position(&lines, 0, 3, 10), (0, 3, false));
        assert_eq!(cursor_position(&lines, 0, 12, 10), (1, 2, false));
        assert_eq!(cursor_position(&lines, 1, 1, 10), (2, 1, false));
        // Escape codes and wide chars count by display width.
        let lines = ["\x1b[1m日本\x1b[0m", "ab"];
        assert_eq!(cursor_position(&lines, 0, 4, 10), (0, 4, false));
        assert_eq!(cursor_position(&lines, 1, 2, 10), (1, 2, false));
    }

    #[test]
    fn a_line_that_fills_the_width_needs_a_row_for_the_cursor() {
        let lines = ["0123456789", "x"];
        // Right after the 10th column: a row of its own after line 0.
        assert_eq!(cursor_position(&lines, 0, 10, 10), (1, 0, true));
        assert_eq!(cursor_position(&lines, 0, 9, 10), (0, 9, false));
        // The same at the end of the last line, and after two full rows.
        let lines = ["ab", "0123456789"];
        assert_eq!(cursor_position(&lines, 1, 10, 10), (2, 0, true));
        let lines = ["01234567890123456789"];
        assert_eq!(cursor_position(&lines, 0, 20, 10), (2, 0, true));
        assert_eq!(cursor_position(&lines, 0, 15, 10), (1, 5, false));
        // An empty line still takes a row.
        assert_eq!(cursor_position(&[""], 0, 0, 10), (0, 0, false));
    }

    #[test]
    fn row_lines_count_nested_messages() {
        let message = Message::error("weak")
            .nest(Message::hint("8 chars").nest(Message::hint("deeper")))
            .nest(Message::hint("a digit"));
        assert_eq!(row_lines(Some((&message, MessageSource::Change))), 4);
        assert_eq!(row_lines(None), 0);
    }

    #[test]
    fn prompt_theme_overrides_app_theme() {
        let custom = ColorfulTheme;
        let handle = ThemeHandle::Prompt(&custom);
        let tokens = custom.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let view = ItemView::new(PromptKind::Select, "item", true, None, None, None, false);

        let (mut via_handle, mut direct) = (String::new(), String::new());
        handle.format_item(&mut via_handle, &ctx, &view).unwrap();
        custom.format_item(&mut direct, &ctx, &view).unwrap();
        assert_eq!(via_handle, direct);
    }
}

#[cfg(all(test, unix))]
mod frame_tests {
    use super::*;
    use crate::{
        prompts::testing::capture,
        theme::{PromptKind, SimpleTheme},
        Status,
    };

    fn items() -> impl Iterator<Item = ItemView<'static>> {
        ["a", "b"].into_iter().enumerate().map(|(i, text)| {
            ItemView::new(PromptKind::Select, text, i == 0, None, None, None, false)
        })
    }

    #[test]
    fn keep_leaves_the_frame_without_its_row_and_legend() {
        let (term, out) = capture();
        let mut render = TermThemeRenderer::new(&term, Some(&SimpleTheme));
        render.set_legend(true, true);
        let note = Message::new(Status::Description, "a note");
        let row = Some((&note, MessageSource::Description));
        render
            .prompt_frame(PromptKind::Select, Some("Pick"), None, false, items(), row)
            .unwrap();
        let drawn = out.text();
        assert!(drawn.contains("a note"), "{drawn:?}");
        assert!(drawn.ends_with("esc back"), "{drawn:?}");
        render.keep().unwrap();
        // The frame is erased and rewritten without the row and the legend.
        let kept = out.text()[drawn.len()..].to_string();
        assert!(kept.contains("Pick:\n> a\n  b"), "{kept:?}");
        assert!(
            !kept.contains("a note") && !kept.contains("esc back"),
            "{kept:?}"
        );
        assert!(kept.ends_with("\r\n"), "{kept:?}");
        assert_eq!((render.rows, render.kept, render.cursor_row), (0, 0, 0));
    }

    #[test]
    fn keep_redraws_committed_frames_too() {
        let (term, out) = capture();
        let mut render = TermThemeRenderer::new(&term, Some(&SimpleTheme));
        let note = Message::new(Status::Description, "a note");
        let row = Some((&note, MessageSource::Description));
        render
            .prompt_frame(PromptKind::Select, Some("One"), None, false, items(), row)
            .unwrap();
        render.commit().unwrap();
        render
            .prompt_frame(PromptKind::Select, Some("Two"), None, false, items(), row)
            .unwrap();
        let before = out.text().len();
        render.keep().unwrap();
        let kept = out.text()[before..].to_string();
        assert!(
            kept.contains("One:\n> a\n  b\nTwo:\n> a\n  b\r\n"),
            "{kept:?}"
        );
        assert!(!kept.contains("a note"), "{kept:?}");
    }

    #[test]
    fn the_legend_takes_a_row_only_when_on_and_the_prompt_has_keys() {
        let (term, _) = capture();
        let mut render = TermThemeRenderer::new(&term, Some(&SimpleTheme));
        assert_eq!(render.legend_lines(PromptKind::Select), 0);
        render.set_legend(true, false);
        assert_eq!(render.legend_lines(PromptKind::Select), 1);
        // Confirm has no keys to list.
        assert_eq!(render.legend_lines(PromptKind::Confirm), 0);
    }

    #[test]
    fn finish_draws_per_done() {
        for (done, expect_line) in [
            (Done::Collapse, true),
            (Done::Keep, false),
            (Done::Erase, false),
        ] {
            let (term, out) = capture();
            let mut render = TermThemeRenderer::new(&term, Some(&SimpleTheme));
            render
                .prompt_frame(PromptKind::Select, Some("Pick"), None, false, items(), None)
                .unwrap();
            let before = out.text().len();
            render
                .finish(done, PromptKind::Select, "Pick", Answer::Text("b"))
                .unwrap();
            let after = out.text()[before..].to_string();
            assert_eq!(
                after.contains("Pick: b\n"),
                expect_line,
                "{done:?}: {after:?}"
            );
            assert_eq!(
                after.contains("> a"),
                done == Done::Keep,
                "{done:?}: {after:?}"
            );
        }
    }

    #[test]
    fn cancel_draws_the_cancelled_line_only_when_the_theme_has_a_value() {
        let (term, out) = capture();
        let mut render = TermThemeRenderer::new(&term, Some(&SimpleTheme));
        let result: Result<()> = render.cancel(PromptKind::Select, Some("Pick"));
        assert!(matches!(result, Err(Error::Cancelled)));
        assert!(!out.text().contains("Pick"));

        let themed =
            SimpleTheme.restyle(|t| t.prompts(|p| p.base(|b| b.cancelled_value("cancelled"))));
        let (term, out) = capture();
        let mut render = TermThemeRenderer::new(&term, Some(&themed));
        let result: Result<()> = render.cancel(PromptKind::Select, Some("Pick"));
        assert!(matches!(result, Err(Error::Cancelled)));
        assert!(out.text().contains("Pick: cancelled\n"), "{:?}", out.text());
        // A list prompt without a question leaves no line.
        let (term, out) = capture();
        let mut render = TermThemeRenderer::new(&term, Some(&themed));
        let _: Result<()> = render.cancel(PromptKind::Select, None);
        assert!(!out.text().contains("cancelled"));
    }
}
