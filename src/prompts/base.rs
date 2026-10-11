//! State and builder methods every prompt shares.
use crate::theme::Theme;

/// What all prompts have in common: their own theme, if any, and whether to report.
///
/// Each prompt embeds one as `base`. Settings left unset (`None`) fall through to
/// [`Options`](crate::Options).
#[derive(Clone)]
pub(crate) struct PromptBase<'a> {
    pub(crate) theme: Option<&'a dyn Theme>,
    pub(crate) report: Option<bool>,
}

impl PromptBase<'_> {
    /// No theme of its own (the renderer then uses the app theme from
    /// [`set_theme`](crate::set_theme)) and nothing set.
    pub(crate) fn new() -> Self {
        PromptBase {
            theme: None,
            report: None,
        }
    }
}

/// Adds the builder methods every prompt shares (`theme`, `report`) and
/// `report_enabled` to a prompt that embeds a [`PromptBase`] as `base`. Use it inside
/// an `impl<'a> Prompt<'a>` block. `$group` is the prompt's field in
/// [`Options`](crate::Options); `$default` is a closure giving the built-in default
/// for `report`.
macro_rules! base_methods {
    ($group:ident, $default:expr) => {
        /// Draws this prompt with `theme` instead of the app theme set with
        /// [`set_theme`](crate::set_theme).
        pub fn theme(mut self, theme: &'a dyn crate::theme::Theme) -> Self {
            self.base.theme = Some(theme);
            self
        }

        /// Indicates whether to report the chosen result after interaction.
        ///
        /// Left unset, the app option applies ([`Options`](crate::Options)); the
        /// built-in default is to report.
        pub fn report(mut self, val: bool) -> Self {
            self.base.report = Some(val);
            self
        }

        fn report_enabled(&self) -> bool {
            let options = crate::options::current();
            options
                .report(self.base.report, options.$group.report)
                .unwrap_or(($default)(self))
        }
    };
}
pub(crate) use base_methods;
