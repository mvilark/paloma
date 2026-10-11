use std::{
    io,
    ops::{Deref, DerefMut},
};

use console::{Key, Term};
use zeroize::Zeroizing;

use crate::{
    options::Done,
    output::message::{IntoMessage, Message, Status},
    prompts::{
        base::{base_methods, PromptBase},
        line::{Edit, LineEditor},
        row::{Keys, Row, Validators},
    },
    theme::{
        render::{Shown, TermThemeRenderer},
        {Answer, PromptKind},
    },
    validate::PasswordValidator,
    Error, Result,
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

/// One entry of a password prompt: the first, or its confirmation.
struct Entry<'p> {
    prompt: &'p str,
    /// The glyph echoed for each typed char, or `None` to echo nothing.
    mask: Option<char>,
    /// The first entry has the row and runs the validators.
    first: bool,
    allow_back: bool,
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
    preset: Option<Zeroizing<String>>,
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

    base_methods!(password);

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

    /// Skips the prompt when `val` is `Some`: it returns that password without needing
    /// a terminal, and draws the answered line (a fixed number of mask characters, see
    /// [`result_len`](Self::result_len)) as the prompt would have left it
    /// ([`done`](Self::done); `Keep` draws like `Collapse`). The preset is not
    /// validated and `on_change` is not called. `None` asks as usual.
    ///
    /// Unattended mode (`general().unattended(true)`) has no password to give, so a
    /// password prompt without a preset returns
    /// [`Error::NoDefault`].
    pub fn preset(mut self, val: Option<String>) -> Self {
        self.preset = val.map(Zeroizing::new);
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
    /// The dialog is rendered on stderr. `Esc` is ignored (see
    /// [`interact_opt`](Self::interact_opt)); `Ctrl+C` returns
    /// [`Error::Cancelled`].
    pub fn interact(self) -> Result<String> {
        self.interact_on(&Term::stderr())
    }

    /// Like [`interact`](Self::interact), but `Esc` goes back: the prompt is erased
    /// and the result is `None`.
    pub fn interact_opt(self) -> Result<Option<String>> {
        self.interact_on_opt(&Term::stderr())
    }

    /// Like [`interact`](Self::interact) but allows a specific terminal to be set.
    pub fn interact_on(self, term: &Term) -> Result<String> {
        Ok(self
            ._interact_on(term, false)?
            .ok_or_else(|| io::Error::other("Quit not allowed in this case"))?)
    }

    /// Like [`interact_opt`](Self::interact_opt) but allows a specific terminal to be
    /// set.
    pub fn interact_on_opt(self, term: &Term) -> Result<Option<String>> {
        self._interact_on(term, true)
    }

    fn _interact_on(mut self, term: &Term, allow_back: bool) -> Result<Option<String>> {
        let options = crate::options::current();
        let done = self.done_mode(&options);
        let result_len = self.result_len.or(options.password.result_len).unwrap_or(8);

        // A preset answers without a terminal; an unattended run has no password.
        if let Some(password) = self.preset.take() {
            if done != Done::Erase {
                let mut render = TermThemeRenderer::new(term, self.base.theme);
                let answer = Answer::Hidden(result_len);
                render.finish_preset(done, PromptKind::Password, &self.prompt, answer)?;
            }
            return Ok(Some((*password).clone()));
        }
        if options.unattended() {
            return Err(Error::NoDefault);
        }

        if !term.is_term() {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "not a terminal").into());
        }

        let mut render = TermThemeRenderer::new(term, self.base.theme);
        render.set_legend(self.legend_enabled(&options), allow_back);

        let masked = self.mask_input.or(options.password.mask).unwrap_or(false);
        let glyph = render
            .look(PromptKind::Password)
            .mask
            .chars()
            .next()
            .unwrap_or('*');
        let mask = self.mask.or_else(|| masked.then_some(glyph));
        let prompt = self.prompt.clone();
        render.set_cursor_shape(PromptKind::Password)?;

        let worker = self.row.worker();
        Keys::run(worker, |keys| loop {
            let entry = Entry {
                prompt: &prompt,
                mask,
                first: true,
                allow_back,
            };
            let Some(password) = self.read(term, &mut render, keys, entry)? else {
                render.back()?;
                return Ok(None);
            };

            if let Some((ref prompt, ref err)) = self.confirmation_prompt.clone() {
                // The first entry stays on screen above the confirmation.
                render.commit()?;
                let entry = Entry {
                    prompt,
                    mask,
                    first: false,
                    allow_back,
                };
                let Some(pw2) = self.read(term, &mut render, keys, entry)? else {
                    render.back()?;
                    return Ok(None);
                };

                if *password != *pw2 {
                    render.erase()?;
                    self.row.invalid(Message::error(err.as_str()));
                    continue;
                }
            }

            render.finish(
                done,
                PromptKind::Password,
                &self.prompt,
                Answer::Hidden(result_len),
            )?;

            return Ok(Some((*password).clone()));
        })
    }

    /// Reads one password key by key, echoing the entry's mask for each character, or
    /// nothing when it has none, until `Enter` submits an acceptable one. The `first`
    /// entry has the row and runs the validators; a refused password stays as typed
    /// and its error shows in the row. `None` when the user left with `Esc` (and
    /// `allow_back`).
    fn read(
        &mut self,
        term: &Term,
        render: &mut TermThemeRenderer,
        keys: &mut Keys<str>,
        entry: Entry,
    ) -> Result<Option<Zeroizing<String>>> {
        let Entry {
            prompt,
            mask,
            first,
            allow_back,
        } = entry;
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

            let Some(key) = keys.next(term, &mut self.row)? else {
                continue;
            };
            match key {
                Key::CtrlC => return render.cancel(PromptKind::Password, Some(&self.prompt)),
                Key::Escape if allow_back => return Ok(None),
                Key::Enter => {
                    let password = Zeroizing::new(editor.text());
                    if password.is_empty() && !self.allow_empty_password {
                        continue;
                    }
                    let validates = first && !self.validators.is_empty();
                    if keys.defer_submit(&mut self.row, validates) {
                        continue;
                    }
                    if first {
                        if let Some(err) = self.validators.check(&password) {
                            self.row.invalid(err);
                            continue;
                        }
                    }
                    return Ok(Some(password));
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
            preset: None,
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

    #[cfg(unix)]
    mod unattended {
        use super::*;
        use crate::{prompts::testing::capture, theme::SimpleTheme};

        #[test]
        fn a_preset_answers_without_a_terminal() {
            let (term, out) = capture();
            let password = Password::new()
                .theme(&SimpleTheme)
                .prompt("Password")
                .preset(Some("hunter2".to_string()))
                .interact_on(&term)
                .unwrap();
            assert_eq!(password, "hunter2");
            assert_eq!(out.text(), "Password: [hidden]\n");
        }

        #[test]
        fn a_preset_draws_as_done_says() {
            for (done, expected) in [
                (Done::Collapse, "Password: [hidden]\n"),
                (Done::Keep, "Password: [hidden]\n"),
                (Done::Erase, ""),
            ] {
                let (term, out) = capture();
                Password::new()
                    .theme(&SimpleTheme)
                    .prompt("Password")
                    .done(done)
                    .preset(Some("x".to_string()))
                    .interact_on_opt(&term)
                    .unwrap();
                assert_eq!(out.text(), expected, "{done:?}");
            }
        }

        #[test]
        fn none_asks_as_usual_and_a_non_terminal_is_an_error() {
            let (term, _) = capture();
            let err = Password::new().preset(None).interact_on(&term);
            assert!(matches!(err, Err(Error::IO(_))));
        }
    }
}
