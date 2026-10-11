use std::{
    io,
    ops::{Deref, DerefMut},
};

use console::{Key, Term};
use zeroize::Zeroizing;

use crate::{
    output::message::{IntoMessage, Message, Status},
    prompts::{
        base::{base_methods, PromptBase},
        line::{Edit, LineEditor},
        row::{Row, Validators},
    },
    theme::{
        render::{Shown, TermThemeRenderer},
        {Answer, PromptKind},
    },
    validate::PasswordValidator,
    Result,
};

/// A line editor holding a secret: its text is overwritten when it is dropped.
struct Secret(LineEditor);

impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl Deref for Secret {
    type Target = LineEditor;

    fn deref(&self) -> &LineEditor {
        &self.0
    }
}

impl DerefMut for Secret {
    fn deref_mut(&mut self) -> &mut LineEditor {
        &mut self.0
    }
}

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
    validators: Validators<'a, String>,
    row: Row<'a, str>,
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

    /// Called each time the typed password changes, with the password. `Some(message)`
    /// shows in the row under the prompt; `None` brings the
    /// [`description`](Self::description) back. paloma passes the password to the
    /// closure and never prints it.
    ///
    /// Only the first entry has a row; the confirmation
    /// ([`confirmation`](Self::confirmation)) has none.
    ///
    /// A later call replaces an earlier closure.
    ///
    /// ```rust,no_run
    /// use paloma::{Message, Password};
    ///
    /// let password = Password::new()
    ///     .prompt("Password")
    ///     .on_change(|pw: &str| {
    ///         (pw.chars().count() < 8).then(|| {
    ///             Message::error("Password too weak").nest(Message::hint("At least 8 characters"))
    ///         })
    ///     })
    ///     .interact()?;
    /// # Ok::<(), paloma::Error>(())
    /// ```
    pub fn on_change(
        mut self,
        f: impl FnMut(&str) -> Option<Message<'static>> + Send + 'a,
    ) -> Self {
        self.row.on_change(f);
        self
    }

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
    pub fn interact_on(mut self, term: &Term) -> Result<String> {
        if !term.is_term() {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "not a terminal").into());
        }

        let mut render = TermThemeRenderer::new(term, self.base.theme);

        let options = crate::options::current();
        let masked = self.mask_input.or(options.password.mask).unwrap_or(false);
        let glyph = render
            .look(PromptKind::Password)
            .mask
            .chars()
            .next()
            .unwrap_or('*');
        let mask = self.mask.or_else(|| masked.then_some(glyph));
        let result_len = self.result_len.or(options.password.result_len).unwrap_or(8);
        let prompt = self.prompt.clone();

        loop {
            let password = self.read(term, &mut render, &prompt, mask, true)?;

            if let Some((ref prompt, ref err)) = self.confirmation_prompt.clone() {
                // The first entry stays on screen above the confirmation.
                render.commit()?;
                let pw2 = self.read(term, &mut render, prompt, mask, false)?;

                if *password != *pw2 {
                    render.erase()?;
                    self.row.invalid(Message::error(err.as_str()));
                    continue;
                }
            }

            render.erase()?;

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

    /// Reads one password key by key, echoing `mask` for each character, or nothing
    /// when `None`, until `Enter` submits an acceptable one. The `first` entry has the
    /// row and runs the validators; a refused password stays as typed and its error
    /// shows in the row.
    fn read(
        &mut self,
        term: &Term,
        render: &mut TermThemeRenderer,
        prompt: &str,
        mask: Option<char>,
        first: bool,
    ) -> Result<Zeroizing<String>> {
        let mut editor = Secret(LineEditor::new(""));
        loop {
            let shown = match mask {
                Some(glyph) => Shown::Mask {
                    before: editor.cursor(),
                    after: editor.len() - editor.cursor(),
                    glyph,
                },
                None => Shown::Hidden,
            };
            let row = if first {
                self.row.current(self.base.description.as_ref())
            } else {
                None
            };
            render.input_frame(
                PromptKind::Password,
                prompt,
                None,
                first && self.row.is_invalid(),
                shown,
                row,
            )?;

            match term.read_key()? {
                Key::Enter => {
                    let password = Zeroizing::new(editor.text());
                    if password.is_empty() && !self.allow_empty_password {
                        continue;
                    }
                    if first {
                        if let Some(err) = self.validators.check(&password) {
                            self.row.invalid(err);
                            continue;
                        }
                    }
                    return Ok(password);
                }
                key => {
                    if editor.handle(&key) == Edit::Changed && first {
                        self.row.changed(&Zeroizing::new(editor.text()));
                    }
                }
            }
        }
    }
}

impl<'a> Password<'a> {
    /// Registers a validator, run when the first entry is submitted (before the
    /// confirmation). An error keeps the prompt open with the password as typed and
    /// shows in the row under it until the password changes. Several validators run
    /// in order; the first error wins.
    ///
    /// The error can be any `Display` value (shown as an error message) or a
    /// [`Message`] of any kind (see [`IntoMessage`]).
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
        V::Err: IntoMessage,
    {
        self.validators.push(move |value: &String| {
            validator
                .validate(value)
                .err()
                .map(|err| err.into_message(Status::Error))
        });
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
            validators: Validators::default(),
            row: Row::default(),
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
