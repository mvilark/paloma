use std::{io, sync::Arc};

use console::{Key, Term};
use zeroize::Zeroizing;

use crate::{
    prompts::base::{base_methods, PromptBase},
    theme::{
        render::TermThemeRenderer,
        {Answer, PromptKind},
    },
    validate::PasswordValidator,
    Result,
};

type PasswordValidatorCallback<'a> = Arc<dyn Fn(&String) -> Option<String> + 'a>;

/// Renders a password input prompt.
///
/// ## Example
///
/// ```rust,no_run
/// use paloma::Password;
///
/// fn main() {
///     let password = Password::new()
///         .prompt("New Password")
///         .confirmation("Confirm password", "Passwords mismatching")
///         .interact()
///         .unwrap();
///
///     println!("Your password length is: {}", password.len());
/// }
/// ```
#[derive(Clone)]
pub struct Password<'a> {
    prompt: String,
    base: PromptBase<'a>,
    allow_empty_password: bool,
    confirmation_prompt: Option<(String, String)>,
    validator: Option<PasswordValidatorCallback<'a>>,
    mask: Option<char>,
    mask_input: Option<bool>,
    result_len: Option<usize>,
}

impl Default for Password<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Password<'a> {
    /// Sets the password input prompt.
    pub fn prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.prompt = prompt.into();
        self
    }

    base_methods!(password, |_| true);

    /// Sets how many mask characters the one-line result shows after the password is
    /// confirmed (default 8). The count is fixed, so the result never reveals the
    /// password's length. Themes decide how the mask is drawn; the default theme
    /// ignores it.
    ///
    /// Left unset, the app option applies ([`Options::password`](crate::Options::password)).
    pub fn result_len(mut self, len: usize) -> Self {
        self.result_len = Some(len);
        self
    }

    /// Sets whether typed characters are echoed as mask characters, drawn with the
    /// theme's `mask` token ([`PromptStyle`](crate::theme::PromptStyle)). Use
    /// [`mask_char`](Self::mask_char) to pick the character yourself.
    ///
    /// Left unset, the app option applies; by default nothing is echoed.
    pub fn mask(mut self, val: bool) -> Self {
        self.mask_input = Some(val);
        self
    }

    /// Enables confirmation prompting.
    pub fn confirmation<A, B>(mut self, prompt: A, mismatch_err: B) -> Self
    where
        A: Into<String>,
        B: Into<String>,
    {
        self.confirmation_prompt = Some((prompt.into(), mismatch_err.into()));
        self
    }

    /// Allows/Disables empty password.
    ///
    /// By default this setting is set to false (i.e. password is not empty).
    pub fn allow_empty_password(mut self, allow_empty_password: bool) -> Self {
        self.allow_empty_password = allow_empty_password;
        self
    }

    /// Sets the character used to mask each typed character.
    ///
    /// By default no mask is used (unless [`mask`](Self::mask) is on) and the input is
    /// fully hidden (nothing is echoed). When a mask is set, one mask character is echoed for every
    /// character typed.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use paloma::Password;
    ///
    /// fn main() {
    ///     let password = Password::new()
    ///         .prompt("Enter password")
    ///         .mask_char('*')
    ///         .interact()
    ///         .unwrap();
    ///
    ///     println!("Your password length is: {}", password.len());
    /// }
    /// ```
    pub fn mask_char(mut self, mask: char) -> Self {
        self.mask = Some(mask);
        self
    }

    /// Enables user interaction and returns the result.
    ///
    /// If the user confirms the result is `Ok()`, `Err()` otherwise.
    /// The dialog is rendered on stderr.
    pub fn interact(self) -> Result<String> {
        self.interact_on(&Term::stderr())
    }

    /// Like [`interact`](Self::interact) but allows a specific terminal to be set.
    pub fn interact_on(self, term: &Term) -> Result<String> {
        if !term.is_term() {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "not a terminal").into());
        }

        let mut render = TermThemeRenderer::new(term, self.base.theme);
        render.set_prompts_reset_height(false);

        let options = crate::options::current();
        let masked = self.mask_input.or(options.password.mask).unwrap_or(false);
        let mask = self
            .mask
            .or_else(|| masked.then(|| render.password_mask_glyph()));
        let result_len = self.result_len.or(options.password.result_len).unwrap_or(8);

        loop {
            let password = Zeroizing::new(self.prompt_password(&mut render, &self.prompt, mask)?);

            if let Some(ref validator) = self.validator {
                if let Some(err) = validator(&password) {
                    render.clear()?;
                    render.input_error(&err)?;
                    continue;
                }
            }

            if let Some((ref prompt, ref err)) = self.confirmation_prompt {
                let pw2 = Zeroizing::new(self.prompt_password(&mut render, prompt, mask)?);

                if *password != *pw2 {
                    render.clear()?;
                    render.input_error(err)?;
                    continue;
                }
            }

            render.clear()?;

            if self.report_enabled() {
                render.answered(
                    PromptKind::Password,
                    &self.prompt,
                    Answer::Hidden(result_len),
                )?;
            }
            term.flush()?;

            return Ok((*password).clone());
        }
    }

    /// Reads one password, echoing `mask` for each character, or nothing when `None`.
    fn prompt_password(
        &self,
        render: &mut TermThemeRenderer,
        prompt: &str,
        mask: Option<char>,
    ) -> Result<String> {
        loop {
            render.password_prompt(prompt)?;
            render.term().flush()?;

            let Some(mask) = mask else {
                let input = render.term().read_secure_line()?;
                render.input_submitted()?;

                if !input.is_empty() || self.allow_empty_password {
                    return Ok(input);
                }

                continue;
            };

            let term = render.term();
            let mut password = String::new();

            loop {
                match term.read_key()? {
                    Key::Char(chr) if !chr.is_ascii_control() => {
                        password.push(chr);
                        term.write_str(mask.encode_utf8(&mut [0; 4]))?;
                        term.flush()?;
                    }
                    Key::Backspace if !password.is_empty() => {
                        password.pop();
                        term.clear_chars(1)?;
                        term.flush()?;
                    }
                    Key::Enter => break,
                    _ => {}
                }
            }

            term.write_line("")?;

            render.input_submitted()?;

            if !password.is_empty() || self.allow_empty_password {
                return Ok(password);
            }
        }
    }
}

impl<'a> Password<'a> {
    /// Registers a validator.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use paloma::Password;
    ///
    /// fn main() {
    ///     let password: String = Password::new()
    ///         .prompt("Enter password")
    ///         .validate(|input: &String| -> Result<(), &str> {
    ///             if input.chars().count() > 8 {
    ///                 Ok(())
    ///             } else {
    ///                 Err("Password must be longer than 8")
    ///             }
    ///         })
    ///         .interact()
    ///         .unwrap();
    /// }
    /// ```
    pub fn validate<V>(mut self, validator: V) -> Self
    where
        V: PasswordValidator + 'a,
        V::Err: ToString,
    {
        let old_validator_func = self.validator.take();

        self.validator = Some(Arc::new(move |value: &String| -> Option<String> {
            if let Some(old) = &old_validator_func {
                if let Some(err) = old(value) {
                    return Some(err);
                }
            }

            match validator.validate(value) {
                Ok(()) => None,
                Err(err) => Some(err.to_string()),
            }
        }));

        self
    }

    /// Creates a password input prompt drawn with the app theme (set with [`set_theme`](crate::set_theme), `SimpleTheme` by default).
    ///
    /// [`theme`](Self::theme) gives this prompt a theme of its own.
    pub fn new() -> Self {
        Self {
            prompt: "".into(),
            base: PromptBase::new(),
            allow_empty_password: false,
            confirmation_prompt: None,
            validator: None,
            mask: None,
            mask_input: None,
            result_len: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone() {
        let password = Password::new().prompt("Enter password");

        let _ = password.clone();
    }
}
