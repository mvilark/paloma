use std::{fmt, io, sync::Arc};

use console::{measure_text_width, Term, TermTarget};

#[cfg(feature = "fuzzy-select")]
use crate::theme::FuzzySelectPromptView;
use crate::{
    terminal::Stream,
    theme::{
        Answer, AnsweredView, ConfirmButtonsView, ConfirmPromptView, Context, InputPromptView,
        ItemView, MessageView, PromptKind, PromptView, Status, Theme, Tokens,
    },
    Result,
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
    text.split('\n')
        .map(|line| measure_text_width(line).max(1).div_ceil(width.max(1)))
        .sum()
}

/// Helper struct to conveniently render a theme.
pub(crate) struct TermThemeRenderer<'a> {
    term: &'a Term,
    theme: ThemeHandle<'a>,
    /// The theme's tokens, read once when the prompt is shown.
    tokens: Tokens,
    /// The stream `term` writes to, so styles follow its terminal.
    stream: Stream,
    height: usize,
    prompt_height: usize,
    prompts_reset_height: bool,
    /// Pauses a running live component (a task's spinner) while the prompt is shown.
    _pause: crate::live::engine::Pause,
    /// An input error waiting to be drawn under the next input prompt.
    pending_error: Option<String>,
    /// An error row is on screen under the input line.
    error_shown: bool,
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
            height: 0,
            prompt_height: 0,
            prompts_reset_height: true,
            pending_error: None,
            error_shown: false,
            _pause: crate::live::engine::Pause::new(),
        }
    }

    #[cfg(feature = "password")]
    pub fn set_prompts_reset_height(&mut self, val: bool) {
        self.prompts_reset_height = val;
    }

    #[cfg(feature = "password")]
    pub fn term(&self) -> &Term {
        self.term
    }

    /// The terminal read a line, so the cursor is now on the row under the input. If
    /// that row holds the input error, it is cleared and reused; otherwise it is a new row.
    #[cfg(feature = "password")]
    pub fn input_submitted(&mut self) -> Result {
        if self.error_shown {
            self.term.clear_line()?;
            self.error_shown = false;
        } else {
            self.height += 1;
        }
        Ok(())
    }

    fn write_formatted_str<
        F: FnOnce(&mut TermThemeRenderer, &mut dyn fmt::Write) -> fmt::Result,
    >(
        &mut self,
        f: F,
    ) -> Result<usize> {
        let mut buf = String::new();
        f(self, &mut buf).map_err(io::Error::other)?;
        self.height += buf.chars().filter(|&x| x == '\n').count();
        self.term.write_str(&buf)?;
        // A prompt may span lines; the cursor sits after the last one.
        Ok(measure_text_width(buf.rsplit('\n').next().unwrap_or("")))
    }

    fn write_formatted_line<
        F: FnOnce(&mut TermThemeRenderer, &mut dyn fmt::Write) -> fmt::Result,
    >(
        &mut self,
        f: F,
    ) -> Result {
        let mut buf = String::new();
        f(self, &mut buf).map_err(io::Error::other)?;
        self.height += rows(&buf, self.term.size().1 as usize);
        Ok(self.term.write_line(&buf)?)
    }

    fn write_formatted_prompt<
        F: FnOnce(&mut TermThemeRenderer, &mut dyn fmt::Write) -> fmt::Result,
    >(
        &mut self,
        f: F,
    ) -> Result {
        self.write_formatted_line(f)?;
        if self.prompts_reset_height {
            self.prompt_height = self.height;
            self.height = 0;
        }
        Ok(())
    }

    /// The context hooks draw with: this renderer's tokens and the term's stream.
    fn ctx(&self) -> Context<'_> {
        Context::new(&self.tokens, self.stream)
    }

    /// The resolved look of one prompt.
    #[cfg(feature = "password")]
    fn look(&self, kind: PromptKind) -> crate::theme::PromptLook {
        self.tokens.get_prompts().resolve(kind, &self.tokens)
    }

    /// The question line of a list prompt, with its paging counter.
    pub fn prompt(
        &mut self,
        kind: PromptKind,
        prompt: &str,
        paging: Option<(usize, usize)>,
    ) -> Result {
        self.write_formatted_prompt(|this, buf| {
            let view = PromptView::new(kind, prompt, paging);
            this.theme.format_prompt(buf, &this.ctx(), &view)
        })
    }

    /// The line an answered prompt leaves.
    pub fn answered(&mut self, kind: PromptKind, prompt: &str, answer: Answer) -> Result {
        self.write_formatted_prompt(|this, buf| {
            let view = AnsweredView::new(kind, prompt, answer);
            this.theme.format_answered(buf, &this.ctx(), &view)
        })
    }

    /// One item of a list prompt.
    pub fn item(
        &mut self,
        kind: PromptKind,
        text: &str,
        active: bool,
        checked: Option<bool>,
        matches: Option<&[usize]>,
    ) -> Result {
        self.write_formatted_line(|this, buf| {
            let view = ItemView::new(kind, text, active, checked, matches);
            this.theme.format_item(buf, &this.ctx(), &view)
        })
    }

    pub fn confirm_prompt(&mut self, prompt: &str, default: Option<bool>) -> Result<usize> {
        self.write_formatted_str(|this, buf| {
            let view = ConfirmPromptView::new(prompt, default, false);
            this.theme.format_confirm_prompt(buf, &this.ctx(), &view)
        })
    }

    /// The question, then the buttons on the line under it.
    pub fn confirm_buttons(&mut self, prompt: &str, yes_focused: bool) -> Result<usize> {
        self.write_formatted_str(|this, buf| {
            let ctx = this.ctx();
            let view = ConfirmPromptView::new(prompt, None, true);
            this.theme.format_confirm_prompt(buf, &ctx, &view)?;
            writeln!(buf)?;
            let view = ConfirmButtonsView::new(yes_focused);
            this.theme.format_confirm_buttons(buf, &ctx, &view)
        })
    }

    #[cfg(feature = "fuzzy-select")]
    pub fn fuzzy_select_prompt(&mut self, prompt: &str, search: &str, cursor: usize) -> Result {
        self.write_formatted_prompt(|this, buf| {
            let view = FuzzySelectPromptView::new(prompt, search, cursor);
            this.theme
                .format_fuzzy_select_prompt(buf, &this.ctx(), &view)
        })
    }

    pub fn input_prompt(&mut self, prompt: &str, default: Option<&str>) -> Result<usize> {
        let len = self.write_formatted_str(|this, buf| {
            let view = InputPromptView::new(PromptKind::Input, prompt, default);
            this.theme.format_input_prompt(buf, &this.ctx(), &view)
        })?;
        self.draw_pending_error(len)?;
        Ok(len)
    }

    /// Draws a pending input error on the row under the input, then puts the cursor
    /// back after the prompt, `len` columns in.
    fn draw_pending_error(&mut self, len: usize) -> Result {
        if let Some(err) = self.pending_error.take() {
            self.write_formatted_str(|this, buf| {
                writeln!(buf)?;
                let view = MessageView::new(Status::Error, &err, 0);
                this.theme.format_message(buf, &this.ctx(), &view)
            })?;
            self.term.move_cursor_up(1)?;
            self.term.write_str("\r")?;
            self.term.move_cursor_right(len)?;
            self.error_shown = true;
        }
        Ok(())
    }

    /// Reports an input error: it is drawn under the input line when the prompt
    /// (input or password) is shown again.
    pub fn input_error(&mut self, err: &str) -> Result {
        self.pending_error = Some(err.to_string());
        Ok(())
    }

    #[cfg(feature = "password")]
    pub fn password_prompt(&mut self, prompt: &str) -> Result<usize> {
        let len = self.write_formatted_str(|this, buf| {
            write!(buf, "\r")?;
            let view = InputPromptView::new(PromptKind::Password, prompt, None);
            this.theme.format_input_prompt(buf, &this.ctx(), &view)
        })?;
        self.draw_pending_error(len)?;
        Ok(len)
    }

    /// The character a masked password shows for each typed character.
    #[cfg(feature = "password")]
    pub fn password_mask_glyph(&self) -> char {
        self.look(PromptKind::Password)
            .mask
            .chars()
            .next()
            .unwrap_or('*')
    }

    pub fn clear(&mut self) -> Result {
        if self.error_shown {
            // The cursor is on the input line; step down so the error row is cleared too.
            self.term.move_cursor_down(1)?;
            self.term.clear_line()?;
            self.error_shown = false;
        }
        self.term
            .clear_last_lines(self.height + self.prompt_height)?;
        self.height = 0;
        self.prompt_height = 0;
        Ok(())
    }

    /// Clears what was drawn since the last prompt line, keeping the prompt.
    pub fn clear_preserve_prompt(&mut self) -> Result {
        self.term.clear_last_lines(self.height)?;
        self.height = 0;
        Ok(())
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
    fn prompt_theme_overrides_app_theme() {
        let custom = ColorfulTheme;
        let handle = ThemeHandle::Prompt(&custom);
        let tokens = custom.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let view = ItemView::new(PromptKind::Select, "item", true, None, None);

        let (mut via_handle, mut direct) = (String::new(), String::new());
        handle.format_item(&mut via_handle, &ctx, &view).unwrap();
        custom.format_item(&mut direct, &ctx, &view).unwrap();
        assert_eq!(via_handle, direct);
    }
}
