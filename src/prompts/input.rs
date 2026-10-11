#[cfg(feature = "history")]
use std::sync::{Arc, Mutex};
use std::{io, str::FromStr};

use console::{Key, Term};

#[cfg(feature = "completion")]
use crate::completion::Completion;
#[cfg(feature = "history")]
use crate::history::History;
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
    validate::InputValidator,
    Result,
};

/// Renders an input prompt.
///
/// ## Example
///
/// ```rust,no_run
/// use paloma::Input;
///
/// fn main() {
///     let name: String = Input::new()
///         .prompt("Your name?")
///         .interact()
///         .unwrap();
///
///     println!("Your name is: {}", name);
/// }
/// ```
///
/// It can also be used with turbofish notation:
///
/// ```rust,no_run
/// use paloma::Input;
///
/// fn main() {
///     let name = Input::<String>::new()
///         .prompt("Your name?")
///         .interact()
///         .unwrap();
///
///     println!("Your name is: {}", name);
/// }
/// ```
///
/// A row under the input can describe it, react to what is typed and show why a
/// value was refused:
///
/// ```rust,no_run
/// use paloma::{Input, Message};
///
/// let name: String = Input::new()
///     .prompt("Username")
///     .description("3 to 16 letters or digits")
///     .placeholder("ada")
///     .on_change(|text: &str| {
///         (text.chars().count() > 16).then(|| Message::warning("Too long"))
///     })
///     .validate(|name: &String| -> Result<(), &str> {
///         if name.chars().count() >= 3 { Ok(()) } else { Err("At least 3 characters") }
///     })
///     .interact()?;
/// # Ok::<(), paloma::Error>(())
/// ```
#[derive(Clone)]
pub struct Input<'a, T> {
    prompt: String,
    post_completion_text: Option<String>,
    default: Option<T>,
    show_default: bool,
    prefill: Option<String>,
    placeholder: Option<String>,
    base: PromptBase<'a>,
    permit_empty: bool,
    validators: Validators<'a, T>,
    row: Row<'a, str>,
    #[cfg(feature = "history")]
    history: Option<Arc<Mutex<&'a mut dyn History<T>>>>,
    #[cfg(feature = "completion")]
    completion: Option<&'a dyn Completion>,
}

impl<T> Default for Input<'_, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> Input<'a, T> {
    /// Sets the input prompt.
    pub fn prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Sets the text shown in place of the prompt once the input is answered. By
    /// default the answered line repeats the prompt.
    pub fn answered_prompt<S: Into<String>>(mut self, post_completion_text: S) -> Self {
        self.post_completion_text = Some(post_completion_text.into());
        self
    }

    /// Returns the text to show in place of the prompt once input is complete.
    fn completion_prompt(&self) -> &str {
        self.post_completion_text.as_deref().unwrap_or(&self.prompt)
    }

    base_methods!(input, |_| true);

    /// Fills the input with editable text when it opens. The user can change or erase
    /// it; pressing Enter right away submits it as the answer. (Unlike a placeholder,
    /// which is only shown while the input is empty and is never the answer.)
    pub fn prefill<S: Into<String>>(mut self, val: S) -> Self {
        self.prefill = Some(val.into());
        self
    }

    /// Shows `text` while the input is empty, in the theme's `placeholder_style`. It
    /// goes away at the first key and is never the answer (unlike
    /// [`prefill`](Self::prefill)).
    pub fn placeholder<S: Into<String>>(mut self, text: S) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    /// Called each time the text changes, with the raw text (before parsing).
    /// `Some(message)` shows in the row under the input; `None` brings the
    /// [`description`](Self::description) back. paloma never prints the value itself.
    ///
    /// A later call replaces an earlier closure.
    pub fn on_change(
        mut self,
        f: impl FnMut(&str) -> Option<Message<'static>> + Send + 'a,
    ) -> Self {
        self.row.on_change(f);
        self
    }

    /// Sets a default.
    ///
    /// Out of the box the prompt does not have a default and will continue
    /// to display until the user inputs something and hits enter. If a default is set the user
    /// can instead accept the default with enter.
    pub fn default(mut self, value: T) -> Self {
        self.default = Some(value);
        self
    }

    /// Enables or disables an empty input
    ///
    /// By default, if there is no default value set for the input, the user must input a non-empty string.
    pub fn allow_empty(mut self, val: bool) -> Self {
        self.permit_empty = val;
        self
    }

    /// Disables or enables the default value display.
    ///
    /// The default behaviour is to append [`default`](#method.default) to the prompt to tell the
    /// user what is the default value.
    ///
    /// This method does not affect existence of default value, only its display in the prompt!
    pub fn show_default(mut self, val: bool) -> Self {
        self.show_default = val;
        self
    }
}

impl<'a, T> Input<'a, T> {
    /// Creates an input prompt drawn with the app theme (set with [`set_theme`](crate::set_theme), `SimpleTheme` by default).
    ///
    /// [`theme`](Self::theme) gives this prompt a theme of its own.
    pub fn new() -> Self {
        Self {
            prompt: "".into(),
            post_completion_text: None,
            default: None,
            show_default: true,
            prefill: None,
            placeholder: None,
            base: PromptBase::new(),
            permit_empty: false,
            validators: Validators::default(),
            row: Row::default(),
            #[cfg(feature = "history")]
            history: None,
            #[cfg(feature = "completion")]
            completion: None,
        }
    }

    /// Enable history processing
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use std::{collections::VecDeque, fmt::Display};
    /// use paloma::{History, Input};
    ///
    /// struct MyHistory {
    ///     history: VecDeque<String>,
    /// }
    ///
    /// impl Default for MyHistory {
    ///     fn default() -> Self {
    ///         MyHistory {
    ///             history: VecDeque::new(),
    ///         }
    ///     }
    /// }
    ///
    /// impl<T: ToString> History<T> for MyHistory {
    ///     fn read(&self, pos: usize) -> Option<String> {
    ///         self.history.get(pos).cloned()
    ///     }
    ///
    ///     fn write(&mut self, val: &T)
    ///     where
    ///     {
    ///         self.history.push_front(val.to_string());
    ///     }
    /// }
    ///
    /// fn main() {
    ///     let mut history = MyHistory::default();
    ///
    ///     let input = Input::<String>::new()
    ///         .history(&mut history)
    ///         .interact()
    ///         .unwrap();
    /// }
    /// ```
    #[cfg(feature = "history")]
    pub fn history<H>(mut self, history: &'a mut H) -> Self
    where
        H: History<T>,
    {
        self.history = Some(Arc::new(Mutex::new(history)));
        self
    }

    /// Enable completion
    #[cfg(feature = "completion")]
    pub fn completion<C>(mut self, completion: &'a C) -> Self
    where
        C: Completion,
    {
        self.completion = Some(completion);
        self
    }
}

impl<'a, T> Input<'a, T>
where
    T: 'a,
{
    /// Registers a validator, run on submit. An error keeps the prompt open with the
    /// text as typed and shows in the row under the input until the text changes.
    /// Several validators run in order; the first error wins.
    ///
    /// The error can be any `Display` value (shown as an error message) or a
    /// [`Message`] of any kind (see [`IntoMessage`]).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use paloma::Input;
    ///
    /// fn main() {
    ///     let mail: String = Input::new()
    ///         .prompt("Enter email")
    ///         .validate(|input: &String| -> Result<(), &str> {
    ///             if input.contains('@') {
    ///                 Ok(())
    ///             } else {
    ///                 Err("This is not a mail address")
    ///             }
    ///         })
    ///         .interact()
    ///         .unwrap();
    /// }
    /// ```
    pub fn validate<V>(mut self, mut validator: V) -> Self
    where
        V: InputValidator<T> + 'a,
        V::Err: IntoMessage,
    {
        self.validators.push(move |value: &T| {
            validator
                .validate(value)
                .err()
                .map(|err| err.into_message(Status::Error))
        });
        self
    }
}

impl<T> Input<'_, T>
where
    T: Clone + ToString + FromStr,
    <T as FromStr>::Err: ToString,
{
    /// Shows the prompt and returns the value once the user submits a valid one.
    ///
    /// paloma reads every key: the cursor can move with `←` / `→` (and `Alt+B` /
    /// `Alt+F` by word), `↑` / `↓` browse the history and `Tab` completes, when those
    /// are set. The dialog is rendered on stderr.
    pub fn interact(self) -> Result<T> {
        self.interact_on(&Term::stderr())
    }

    /// Like [`interact`](Self::interact) but allows a specific terminal to be set.
    pub fn interact_on(mut self, term: &Term) -> Result<T> {
        if !term.is_term() {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "not a terminal").into());
        }

        let mut render = TermThemeRenderer::new(term, self.base.theme);
        let default_string = self.default.as_ref().map(ToString::to_string);
        let shown_default = default_string.as_deref().filter(|_| self.show_default);
        let mut editor = LineEditor::new(self.prefill.as_deref().unwrap_or(""));
        #[cfg(feature = "history")]
        let mut hist_pos = 0;

        loop {
            let (before, after) = (editor.before_cursor(), editor.after_cursor());
            let shown = match &self.placeholder {
                Some(placeholder) if editor.is_empty() => Shown::Placeholder(placeholder),
                _ => Shown::Text {
                    before: &before,
                    after: &after,
                },
            };
            render.input_frame(
                PromptKind::Input,
                &self.prompt,
                shown_default,
                self.row.is_invalid(),
                shown,
                self.row.current(self.base.description.as_ref()),
            )?;

            let changed = match term.read_key()? {
                Key::Enter => {
                    if let Some(value) = self.submit(&mut render, &editor.text())? {
                        return Ok(value);
                    }
                    false
                }
                #[cfg(feature = "completion")]
                Key::ArrowRight if editor.at_end() => self.complete(&mut editor),
                #[cfg(feature = "completion")]
                Key::Tab => self.complete(&mut editor),
                #[cfg(feature = "history")]
                Key::ArrowUp => match &self.history {
                    Some(history) => {
                        let previous = history.lock().unwrap().read(hist_pos);
                        match previous {
                            Some(previous) => {
                                hist_pos += 1;
                                editor.set(&previous)
                            }
                            None => false,
                        }
                    }
                    None => false,
                },
                #[cfg(feature = "history")]
                Key::ArrowDown => match &self.history {
                    Some(history) => {
                        let mut changed = editor.set("");
                        // Step back over the entry shown now, then show the one before.
                        if let Some(pos) = hist_pos.checked_sub(1) {
                            hist_pos = pos;
                            if let Some(pos) = pos.checked_sub(1) {
                                if let Some(previous) = history.lock().unwrap().read(pos) {
                                    changed = editor.set(&previous) || changed;
                                }
                            }
                        }
                        changed
                    }
                    None => false,
                },
                key => editor.handle(&key) == Edit::Changed,
            };
            if changed {
                self.row.changed(&editor.text());
            }
        }
    }

    /// Replaces the text with its completion, if there is one. Returns whether the text
    /// changed.
    #[cfg(feature = "completion")]
    fn complete(&self, editor: &mut LineEditor) -> bool {
        match self.completion.and_then(|c| c.get(&editor.text())) {
            Some(completed) => editor.set(&completed),
            None => false,
        }
    }

    /// Handles `Enter` with `input` typed: the answer, or `None` when the prompt stays
    /// open (a refused value shows its error in the row).
    fn submit(&mut self, render: &mut TermThemeRenderer, input: &str) -> Result<Option<T>> {
        if input.is_empty() {
            if let Some(default) = &self.default {
                if let Some(err) = self.validators.check(default) {
                    self.row.invalid(err);
                    return Ok(None);
                }
                render.erase()?;
                if self.report_enabled() {
                    let answer = default.to_string();
                    render.answered(
                        PromptKind::Input,
                        self.completion_prompt(),
                        Answer::Text(&answer),
                    )?;
                }
                return Ok(Some(default.clone()));
            } else if !self.permit_empty {
                return Ok(None);
            }
        }

        match input.parse::<T>() {
            Ok(value) => {
                #[cfg(feature = "history")]
                if let Some(history) = &mut self.history {
                    history.lock().unwrap().write(&value);
                }

                if let Some(err) = self.validators.check(&value) {
                    self.row.invalid(err);
                    return Ok(None);
                }

                render.erase()?;
                if self.report_enabled() {
                    render.answered(
                        PromptKind::Input,
                        self.completion_prompt(),
                        Answer::Text(input),
                    )?;
                }
                Ok(Some(value))
            }
            Err(err) => {
                self.row.invalid(Message::error(err.to_string()));
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone() {
        let input = Input::<String>::new().prompt("Your name");

        let _ = input.clone();
    }

    #[test]
    fn validators_chain_and_take_messages() {
        let input = Input::<String>::new()
            .validate(|s: &String| if s.is_empty() { Err("empty") } else { Ok(()) })
            .validate(|s: &String| -> std::result::Result<(), Message<'static>> {
                if s.len() < 3 {
                    Err(Message::warning("short").nest(Message::hint("3 or more")))
                } else {
                    Ok(())
                }
            });
        let err = input.validators.check(&String::new()).unwrap();
        assert_eq!((err.status(), err.text()), (Status::Error, "empty"));
        let err = input.validators.check(&"ab".to_string()).unwrap();
        assert_eq!((err.status(), err.children().len()), (Status::Warning, 1));
        assert!(input.validators.check(&"abc".to_string()).is_none());
    }

    #[test]
    fn test_completion_prompt() {
        let input = Input::<String>::new().prompt("Your name");
        assert_eq!(input.completion_prompt(), "Your name");

        let input = input.answered_prompt("Name").default("Alice".to_string());
        assert_eq!(input.completion_prompt(), "Name");
    }
}
