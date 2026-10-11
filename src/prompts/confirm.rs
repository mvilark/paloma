use std::io;

use console::{Key, Term};

use crate::{
    options::Done,
    output::message::Message,
    prompts::{
        base::{base_methods, PromptBase},
        row::Row,
    },
    theme::{
        render::TermThemeRenderer,
        {Answer, PromptKind},
    },
    Error, Result,
};

/// Renders a confirm prompt.
///
/// ## Example
///
/// ```rust,no_run
/// use paloma::Confirm;
///
/// fn main() {
///     let confirmation = Confirm::new()
///         .prompt("Do you want to continue?")
///         .interact()
///         .unwrap();
///
///     if confirmation {
///         println!("Looks like you want to continue");
///     } else {
///         println!("nevermind then :(");
///     }
/// }
/// ```
#[derive(Clone)]
pub struct Confirm<'a> {
    prompt: String,
    default: Option<bool>,
    show_default: bool,
    choices_as_buttons: Option<bool>,
    on_shortcut: Option<bool>,
    preset: Option<bool>,
    base: PromptBase<'a>,
    row: Row<'a, bool>,
}

impl Default for Confirm<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Confirm<'a> {
    /// Sets the confirm prompt.
    pub fn prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.prompt = prompt.into();
        self
    }

    base_methods!(confirm);

    /// Shows the choices as `Yes` / `No` buttons on the line under the question
    /// (`true`), or as the classic `(y/n)` text after it (`false`, the default).
    ///
    /// With buttons, `←`, `→` and `Tab` move the focus and `Enter` picks it.
    ///
    /// Left unset, the app option applies ([`Options::confirm`](crate::Options::confirm)).
    pub fn choices_as_buttons(mut self, val: bool) -> Self {
        self.choices_as_buttons = Some(val);
        self
    }

    /// Sets whether `y` / `n` answer at once.
    ///
    /// When `true` (the default), the prompt reacts to each keystroke: `y`, `n`, or
    /// `Enter` to accept the default. When `false`, `y` and `n` only select the
    /// choice and the user hits `Enter` to submit. Works with and without buttons.
    ///
    /// Left unset, the app option applies.
    pub fn on_shortcut(mut self, val: bool) -> Self {
        self.on_shortcut = Some(val);
        self
    }

    /// Called each time the focused or chosen answer changes (`y` / `n` without
    /// [`on_shortcut`](Self::on_shortcut), or a button getting the focus), with that
    /// answer. `Some(message)` shows in the row under the prompt; `None` brings the
    /// [`description`](Self::description) back.
    ///
    /// A later call replaces an earlier closure.
    pub fn on_change(
        mut self,
        f: impl FnMut(&bool) -> Option<Message<'static>> + Send + 'a,
    ) -> Self {
        self.row.on_change(f);
        self
    }

    /// Sets a default.
    ///
    /// Out of the box the prompt does not have a default and will continue
    /// to display until the user inputs something and hits enter. If a default is set the user
    /// can instead accept the default with enter. With `general().unattended(true)` the
    /// prompt answers with it.
    pub fn default(mut self, val: bool) -> Self {
        self.default = Some(val);
        self
    }

    /// Skips the prompt when `val` is `Some`: it returns that answer without needing a
    /// terminal, and draws the answered line as the prompt would have left it
    /// ([`done`](Self::done); `Keep` draws like `Collapse`). `on_change` is not called.
    /// `None` asks as usual.
    pub fn preset(mut self, val: Option<bool>) -> Self {
        self.preset = val;
        self
    }

    /// Disables or enables the default value display.
    ///
    /// The default is to append the default value to the prompt to tell the user.
    pub fn show_default(mut self, val: bool) -> Self {
        self.show_default = val;
        self
    }

    /// Enables user interaction and returns the result.
    ///
    /// The dialog is rendered on stderr.
    ///
    /// Result contains `bool` if user answered "yes" or "no" or `default` (configured in [`default`](Self::default) if pushes enter.
    /// This unlike [`interact_opt`](Self::interact_opt) does not allow to quit with 'Esc' or 'q'.
    /// `Ctrl+C` returns [`Error::Cancelled`].
    #[inline]
    pub fn interact(self) -> Result<bool> {
        self.interact_on(&Term::stderr())
    }

    /// Enables user interaction and returns the result.
    ///
    /// The dialog is rendered on stderr.
    ///
    /// Result contains `Some(bool)` if user answered "yes" or "no" or `Some(default)` (configured in [`default`](Self::default)) if pushes enter,
    /// or `None` if user went back with 'Esc' or 'q' (the prompt is erased).
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use paloma::Confirm;
    ///
    /// fn main() {
    ///     let confirmation = Confirm::new()
    ///         .interact_opt()
    ///         .unwrap();
    ///
    ///     match confirmation {
    ///         Some(answer) => println!("User answered {}", if answer { "yes" } else { "no " }),
    ///         None => println!("User did not answer")
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn interact_opt(self) -> Result<Option<bool>> {
        self.interact_on_opt(&Term::stderr())
    }

    /// Like [`interact`](Self::interact) but allows a specific terminal to be set.
    #[inline]
    pub fn interact_on(self, term: &Term) -> Result<bool> {
        Ok(self
            ._interact_on(term, false)?
            .ok_or_else(|| io::Error::other("Quit not allowed in this case"))?)
    }

    /// Like [`interact_opt`](Self::interact_opt) but allows a specific terminal to be set.
    #[inline]
    pub fn interact_on_opt(self, term: &Term) -> Result<Option<bool>> {
        self._interact_on(term, true)
    }

    fn _interact_on(mut self, term: &Term, allow_back: bool) -> Result<Option<bool>> {
        let options = crate::options::current();
        let done = self.done_mode(&options);

        // A preset or an unattended run answers without a terminal.
        let given = match self.preset {
            Some(answer) => Some(answer),
            None if options.unattended() => Some(self.default.ok_or(Error::NoDefault)?),
            None => None,
        };
        if let Some(answer) = given {
            if done != Done::Erase {
                let mut render = TermThemeRenderer::new(term, self.base.theme);
                let line = Answer::Bool(answer);
                render.finish_preset(done, PromptKind::Confirm, &self.prompt, line)?;
            }
            return Ok(Some(answer));
        }

        if !term.is_term() {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "not a terminal").into());
        }

        let mut render = TermThemeRenderer::new(term, self.base.theme);
        render.set_legend(self.legend_enabled(&options), allow_back);

        let wait_for_newline = !self
            .on_shortcut
            .or(options.confirm.on_shortcut)
            .unwrap_or(true);
        let buttons = self
            .choices_as_buttons
            .or(options.confirm.choices_as_buttons)
            .unwrap_or(false);

        let answer = if buttons {
            self.read_buttons(term, &mut render, allow_back, wait_for_newline)?
        } else {
            self.read_choice(term, &mut render, allow_back, wait_for_newline)?
        };
        match answer {
            Some(yes) => {
                render.finish(done, PromptKind::Confirm, &self.prompt, Answer::Bool(yes))?;
                Ok(Some(yes))
            }
            None => {
                render.back()?;
                Ok(None)
            }
        }
    }

    /// The classic layout: the answer is typed (`y` / `n`). `None` when the user left
    /// with `Esc`.
    fn read_choice(
        &mut self,
        term: &Term,
        render: &mut TermThemeRenderer,
        allow_back: bool,
        wait_for_newline: bool,
    ) -> Result<Option<bool>> {
        let default_if_show = if self.show_default {
            self.default
        } else {
            None
        };

        term.hide_cursor()?;

        // With `wait_for_newline`, `y` and `n` only select; `Enter` submits.
        let mut value = default_if_show;
        loop {
            self.draw(render, value, None)?;
            let new = match term.read_key_raw()? {
                Key::CtrlC => return render.cancel(PromptKind::Confirm, Some(&self.prompt)),
                Key::Char('y') | Key::Char('Y') => true,
                Key::Char('n') | Key::Char('N') => false,
                Key::Enter => match value.or(self.default) {
                    Some(answer) => return Ok(Some(answer)),
                    None => continue,
                },
                Key::Escape | Key::Char('q') if allow_back => return Ok(None),
                _ => continue,
            };
            if !wait_for_newline {
                return Ok(Some(new));
            }
            if value != Some(new) {
                self.row.changed(&new);
            }
            value = Some(new);
        }
    }

    /// Draws the prompt showing `value` (the classic layout), or the buttons with the
    /// focus on `Yes` or `No`, and the row under it.
    fn draw(
        &self,
        render: &mut TermThemeRenderer,
        value: Option<bool>,
        buttons: Option<bool>,
    ) -> Result {
        render.confirm_frame(
            &self.prompt,
            value,
            self.row.is_invalid(),
            buttons,
            self.row.current(self.base.description.as_ref()),
        )
    }
}

impl<'a> Confirm<'a> {
    /// The button layout: the question with `Yes` / `No` buttons on the line under it.
    ///
    /// Left/right arrows and `Tab` move the focus and `Enter` picks the focused button.
    /// `y` / `n` pick at once, or only move the focus with
    /// [`on_shortcut(false)`](Self::on_shortcut). `None` when the user left with
    /// `Esc`.
    fn read_buttons(
        &mut self,
        term: &Term,
        render: &mut TermThemeRenderer,
        allow_back: bool,
        wait_for_newline: bool,
    ) -> Result<Option<bool>> {
        let mut yes_focused = self.default.unwrap_or(true);
        term.hide_cursor()?;
        loop {
            self.draw(render, None, Some(yes_focused))?;
            let focused = yes_focused;
            match term.read_key_raw()? {
                Key::CtrlC => return render.cancel(PromptKind::Confirm, Some(&self.prompt)),
                Key::ArrowLeft | Key::ArrowRight | Key::Tab => yes_focused = !yes_focused,
                Key::Char('y') | Key::Char('Y') if wait_for_newline => yes_focused = true,
                Key::Char('n') | Key::Char('N') if wait_for_newline => yes_focused = false,
                Key::Char('y') | Key::Char('Y') => return Ok(Some(true)),
                Key::Char('n') | Key::Char('N') => return Ok(Some(false)),
                Key::Enter => return Ok(Some(yes_focused)),
                Key::Escape | Key::Char('q') if allow_back => return Ok(None),
                _ => {}
            }
            if yes_focused != focused {
                self.row.changed(&yes_focused);
            }
        }
    }
}

impl<'a> Confirm<'a> {
    /// Creates a confirm prompt drawn with the app theme (set with [`set_theme`](crate::set_theme), `SimpleTheme` by default).
    ///
    /// [`theme`](Self::theme) gives this prompt a theme of its own.
    pub fn new() -> Self {
        Self {
            prompt: "".into(),
            default: None,
            show_default: true,
            choices_as_buttons: None,
            on_shortcut: None,
            preset: None,
            base: PromptBase::new(),
            row: Row::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone() {
        let confirm = Confirm::new().prompt("Do you want to continue?");

        let _ = confirm.clone();
    }

    #[cfg(unix)]
    mod unattended {
        use super::*;
        use crate::{prompts::testing::capture, theme::SimpleTheme};

        #[test]
        fn a_preset_answers_without_a_terminal() {
            for (preset, line) in [(true, "Go? yes\n"), (false, "Go? no\n")] {
                let (term, out) = capture();
                let answer = Confirm::new()
                    .theme(&SimpleTheme)
                    .prompt("Go?")
                    .preset(Some(preset))
                    .interact_on(&term)
                    .unwrap();
                assert_eq!(answer, preset);
                assert_eq!(out.text(), line);
            }
        }

        #[test]
        fn a_preset_draws_as_done_says() {
            for (done, expected) in [
                (Done::Collapse, "Go? yes\n"),
                (Done::Keep, "Go? yes\n"),
                (Done::Erase, ""),
            ] {
                let (term, out) = capture();
                let answer = Confirm::new()
                    .theme(&SimpleTheme)
                    .prompt("Go?")
                    .done(done)
                    .preset(Some(true))
                    .interact_on_opt(&term)
                    .unwrap();
                assert_eq!(answer, Some(true));
                assert_eq!(out.text(), expected, "{done:?}");
            }
        }

        #[test]
        fn none_asks_as_usual_and_a_non_terminal_is_an_error() {
            let (term, out) = capture();
            let err = Confirm::new().prompt("Go?").preset(None).interact_on(&term);
            assert!(matches!(err, Err(Error::IO(_))));
            assert_eq!(out.text(), "");
        }
    }
}
