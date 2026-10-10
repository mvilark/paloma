//! App-wide behavior options set by [`set_options`].
//!
//! Themes decide how prompts look; options decide how they work. Settings are
//! grouped by prompt, and a few shared ones live in [`general`](Options::general):
//!
//! ```rust,no_run
//! use paloma::{ConfirmStyle, Options};
//!
//! paloma::set_options(
//!     Options::new()
//!         .general(|g| g.report(true))
//!         .confirm(|c| c.style(ConfirmStyle::Buttons).on_shortcut(false))
//!         .password(|p| p.mask(true).result_len(8))
//!         .select(|s| s.report(false)),
//! );
//! ```
//!
//! A setting is resolved in this order: the prompt's own builder method, the
//! prompt's group here, [`general`](Options::general), the built-in default.
use std::sync::RwLock;

static APP_OPTIONS: RwLock<Option<Options>> = RwLock::new(None);

/// Sets the behavior options for every prompt in the app.
///
/// Calling this again replaces the options; prompts read them when they are shown.
pub fn set_options(options: Options) {
    let mut guard = APP_OPTIONS.write().unwrap_or_else(|e| e.into_inner());
    *guard = Some(options);
}

/// Returns the app options, or the defaults if none were set.
pub(crate) fn current() -> Options {
    let guard = APP_OPTIONS.read().unwrap_or_else(|e| e.into_inner());
    guard.clone().unwrap_or_default()
}

/// How a [`Confirm`](crate::Confirm) shows its choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmStyle {
    /// The classic inline hint, such as `Continue? [y/n]`.
    Inline,
    /// `Yes` / `No` buttons on the line under the question. `←`, `→` and `Tab` move
    /// the focus and `Enter` picks it.
    Buttons,
}

/// Settings that apply to every prompt unless the prompt's group or the prompt itself
/// sets them.
#[derive(Clone, Default, Debug)]
pub struct GeneralOptions {
    pub(crate) report: Option<bool>,
}

impl GeneralOptions {
    /// Whether a prompt prints its one-line result after it is answered.
    pub fn report(mut self, val: bool) -> Self {
        self.report = Some(val);
        self
    }
}

/// Settings for prompts that only have the shared ones: `Input`, `Select`,
/// `MultiSelect`, `Sort` and `FuzzySelect`.
#[derive(Clone, Default, Debug)]
pub struct PromptOptions {
    pub(crate) report: Option<bool>,
}

impl PromptOptions {
    /// Whether this prompt prints its one-line result after it is answered.
    pub fn report(mut self, val: bool) -> Self {
        self.report = Some(val);
        self
    }
}

/// Settings for [`Confirm`](crate::Confirm).
#[derive(Clone, Default, Debug)]
pub struct ConfirmOptions {
    pub(crate) report: Option<bool>,
    pub(crate) style: Option<ConfirmStyle>,
    pub(crate) on_shortcut: Option<bool>,
}

impl ConfirmOptions {
    /// Whether the prompt prints its one-line result after it is answered.
    pub fn report(mut self, val: bool) -> Self {
        self.report = Some(val);
        self
    }

    /// Inline hint or buttons. The default is [`ConfirmStyle::Inline`].
    pub fn style(mut self, style: ConfirmStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Whether `y` / `n` answer at once (`true`, the default) or only select the
    /// choice so the user hits `Enter` to submit (`false`). Works in both styles.
    pub fn on_shortcut(mut self, val: bool) -> Self {
        self.on_shortcut = Some(val);
        self
    }
}

/// Settings for [`Password`](crate::Password).
#[derive(Clone, Default, Debug)]
pub struct PasswordOptions {
    pub(crate) report: Option<bool>,
    pub(crate) mask: Option<bool>,
    pub(crate) result_len: Option<usize>,
}

impl PasswordOptions {
    /// Whether the prompt prints its one-line result after it is answered.
    pub fn report(mut self, val: bool) -> Self {
        self.report = Some(val);
        self
    }

    /// Whether typed characters are echoed as mask characters (`true`) or not at all
    /// (`false`, the default). The theme picks the character with
    /// [`password_mask_glyph`](crate::theme::Theme::password_mask_glyph).
    pub fn mask(mut self, val: bool) -> Self {
        self.mask = Some(val);
        self
    }

    /// How many mask characters the one-line result shows, whatever the password's
    /// length. The default is 8.
    pub fn result_len(mut self, len: usize) -> Self {
        self.result_len = Some(len);
        self
    }
}

/// App-wide behavior options. See the [module docs](self).
#[derive(Clone, Default, Debug)]
pub struct Options {
    pub(crate) general: GeneralOptions,
    pub(crate) confirm: ConfirmOptions,
    pub(crate) password: PasswordOptions,
    pub(crate) input: PromptOptions,
    pub(crate) select: PromptOptions,
    pub(crate) multi_select: PromptOptions,
    pub(crate) sort: PromptOptions,
    pub(crate) fuzzy_select: PromptOptions,
}

impl Options {
    /// Creates options with nothing set, so every prompt uses its built-in defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Settings shared by all prompts.
    pub fn general(mut self, f: impl FnOnce(GeneralOptions) -> GeneralOptions) -> Self {
        self.general = f(self.general);
        self
    }

    /// Settings for [`Confirm`](crate::Confirm).
    pub fn confirm(mut self, f: impl FnOnce(ConfirmOptions) -> ConfirmOptions) -> Self {
        self.confirm = f(self.confirm);
        self
    }

    /// Settings for [`Password`](crate::Password).
    pub fn password(mut self, f: impl FnOnce(PasswordOptions) -> PasswordOptions) -> Self {
        self.password = f(self.password);
        self
    }

    /// Settings for [`Input`](crate::Input).
    pub fn input(mut self, f: impl FnOnce(PromptOptions) -> PromptOptions) -> Self {
        self.input = f(self.input);
        self
    }

    /// Settings for [`Select`](crate::Select).
    pub fn select(mut self, f: impl FnOnce(PromptOptions) -> PromptOptions) -> Self {
        self.select = f(self.select);
        self
    }

    /// Settings for [`MultiSelect`](crate::MultiSelect).
    pub fn multi_select(mut self, f: impl FnOnce(PromptOptions) -> PromptOptions) -> Self {
        self.multi_select = f(self.multi_select);
        self
    }

    /// Settings for [`Sort`](crate::Sort).
    pub fn sort(mut self, f: impl FnOnce(PromptOptions) -> PromptOptions) -> Self {
        self.sort = f(self.sort);
        self
    }

    /// Settings for [`FuzzySelect`](crate::FuzzySelect).
    pub fn fuzzy_select(mut self, f: impl FnOnce(PromptOptions) -> PromptOptions) -> Self {
        self.fuzzy_select = f(self.fuzzy_select);
        self
    }

    /// Resolves `report`: the prompt's own value, then its group, then `general`.
    pub(crate) fn report(&self, own: Option<bool>, group: Option<bool>) -> Option<bool> {
        own.or(group).or(self.general.report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_resolves_prompt_then_group_then_general() {
        let o = Options::new()
            .general(|g| g.report(false))
            .select(|s| s.report(true));
        assert_eq!(o.report(None, o.select.report), Some(true));
        assert_eq!(o.report(None, o.input.report), Some(false));
        assert_eq!(o.report(Some(true), o.input.report), Some(true));
        assert_eq!(Options::new().report(None, None), None);
    }
}
