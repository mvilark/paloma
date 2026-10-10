use std::io;

use console::{Key, Term};

use crate::{
    options::ConfirmStyle,
    prompts::base::{base_methods, PromptBase},
    theme::render::TermThemeRenderer,
    Result,
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
///         .label("Do you want to continue?")
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
    style: Option<ConfirmStyle>,
    on_shortcut: Option<bool>,
    base: PromptBase<'a>,
    show_choices: bool,
}

impl Default for Confirm<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Confirm<'a> {
    /// Sets the confirm prompt.
    pub fn label<S: Into<String>>(mut self, prompt: S) -> Self {
        self.prompt = prompt.into();
        self
    }

    base_methods!(confirm, |_| true);

    /// Sets how the choices are shown: an inline hint or buttons under the question.
    ///
    /// Left unset, the app option applies ([`Options::confirm`](crate::Options::confirm)),
    /// then [`ConfirmStyle::Inline`].
    pub fn style(mut self, style: ConfirmStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets whether `y` / `n` answer at once.
    ///
    /// When `true` (the default), the prompt reacts to each keystroke: `y`, `n`, or
    /// `Enter` to accept the default. When `false`, `y` and `n` only select the
    /// choice and the user hits `Enter` to submit. Works in both styles.
    ///
    /// Left unset, the app option applies.
    pub fn on_shortcut(mut self, val: bool) -> Self {
        self.on_shortcut = Some(val);
        self
    }

    /// Sets a default.
    ///
    /// Out of the box the prompt does not have a default and will continue
    /// to display until the user inputs something and hits enter. If a default is set the user
    /// can instead accept the default with enter.
    pub fn default(mut self, val: bool) -> Self {
        self.default = Some(val);
        self
    }

    /// Disables or enables the default value display.
    ///
    /// The default is to append the default value to the prompt to tell the user.
    pub fn show_default(mut self, val: bool) -> Self {
        self.show_default = val;
        self
    }

    /// Disables or enables the hint display (e.g. `[y/n]`).
    ///
    /// The default is to show the hint.
    pub fn show_choices(mut self, val: bool) -> Self {
        self.show_choices = val;
        self
    }

    /// Enables user interaction and returns the result.
    ///
    /// The dialog is rendered on stderr.
    ///
    /// Result contains `bool` if user answered "yes" or "no" or `default` (configured in [`default`](Self::default) if pushes enter.
    /// This unlike [`interact_opt`](Self::interact_opt) does not allow to quit with 'Esc' or 'q'.
    #[inline]
    pub fn interact(self) -> Result<bool> {
        self.interact_on(&Term::stderr())
    }

    /// Enables user interaction and returns the result.
    ///
    /// The dialog is rendered on stderr.
    ///
    /// Result contains `Some(bool)` if user answered "yes" or "no" or `Some(default)` (configured in [`default`](Self::default)) if pushes enter,
    /// or `None` if user cancelled with 'Esc' or 'q'.
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

    fn _interact_on(self, term: &Term, allow_quit: bool) -> Result<Option<bool>> {
        if !term.is_term() {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "not a terminal").into());
        }

        let mut render = TermThemeRenderer::new(term, self.base.theme);

        let options = crate::options::current();
        let wait_for_newline = !self
            .on_shortcut
            .or(options.confirm.on_shortcut)
            .unwrap_or(true);
        let style = self
            .style
            .or(options.confirm.style)
            .unwrap_or(ConfirmStyle::Inline);
        if style == ConfirmStyle::Buttons {
            return self.interact_buttons(term, render, allow_quit, wait_for_newline);
        }

        let default_if_show = if self.show_default {
            self.default
        } else {
            None
        };

        if self.show_choices {
            render.confirm_prompt(&self.prompt, default_if_show)?;
        } else {
            render.confirm_prompt_no_hint(&self.prompt)?;
        }

        term.hide_cursor()?;
        term.flush()?;

        let rv;

        if wait_for_newline {
            // Waits for user input and for the user to hit the Enter key
            // before validation.
            let mut value = default_if_show;

            loop {
                let input = term.read_key()?;

                match input {
                    Key::Char('y') | Key::Char('Y') => {
                        value = Some(true);
                    }
                    Key::Char('n') | Key::Char('N') => {
                        value = Some(false);
                    }
                    Key::Enter => {
                        if !allow_quit {
                            value = value.or(self.default);
                        }

                        if value.is_some() || allow_quit {
                            rv = value;
                            break;
                        }
                        continue;
                    }
                    Key::Escape | Key::Char('q') if allow_quit => {
                        value = None;
                    }
                    _ => {
                        continue;
                    }
                };

                term.clear_line()?;
                render.confirm_prompt(&self.prompt, value)?;
            }
        } else {
            // Default behavior: matches continuously on every keystroke,
            // and does not wait for user to hit the Enter key.
            loop {
                let input = term.read_key()?;
                let value = match input {
                    Key::Char('y') | Key::Char('Y') => Some(true),
                    Key::Char('n') | Key::Char('N') => Some(false),
                    Key::Enter if self.default.is_some() => Some(self.default.unwrap()),
                    Key::Escape | Key::Char('q') if allow_quit => None,
                    _ => {
                        continue;
                    }
                };

                rv = value;
                break;
            }
        }

        term.clear_line()?;
        if self.report_enabled() {
            render.confirm_prompt_selection(&self.prompt, rv)?;
        }
        term.show_cursor()?;
        term.flush()?;

        Ok(rv)
    }
}

impl<'a> Confirm<'a> {
    /// The button layout: the question with `Yes` / `No` buttons on the line under it.
    ///
    /// Left/right arrows and `Tab` move the focus and `Enter` picks the focused button.
    /// `y` / `n` pick at once, or only move the focus with
    /// [`on_shortcut(false)`](Self::on_shortcut). The whole prompt is erased
    /// afterwards.
    fn interact_buttons(
        self,
        term: &Term,
        mut render: TermThemeRenderer,
        allow_quit: bool,
        wait_for_newline: bool,
    ) -> Result<Option<bool>> {
        let mut yes_focused = self.default.unwrap_or(true);
        term.hide_cursor()?;
        let rv = loop {
            render.confirm_buttons(&self.prompt, yes_focused)?;
            term.flush()?;
            let picked = match term.read_key()? {
                Key::ArrowLeft | Key::ArrowRight | Key::Tab => {
                    yes_focused = !yes_focused;
                    None
                }
                Key::Char('y') | Key::Char('Y') if wait_for_newline => {
                    yes_focused = true;
                    None
                }
                Key::Char('n') | Key::Char('N') if wait_for_newline => {
                    yes_focused = false;
                    None
                }
                Key::Char('y') | Key::Char('Y') => Some(Some(true)),
                Key::Char('n') | Key::Char('N') => Some(Some(false)),
                Key::Enter => Some(Some(yes_focused)),
                Key::Escape | Key::Char('q') if allow_quit => Some(None),
                _ => None,
            };
            term.clear_line()?;
            render.clear()?;
            if let Some(rv) = picked {
                break rv;
            }
        };
        if self.report_enabled() {
            render.confirm_prompt_selection(&self.prompt, rv)?;
        }
        term.show_cursor()?;
        term.flush()?;
        Ok(rv)
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
            style: None,
            on_shortcut: None,
            base: PromptBase::new(),
            show_choices: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone() {
        let confirm = Confirm::new().label("Do you want to continue?");

        let _ = confirm.clone();
    }
}
