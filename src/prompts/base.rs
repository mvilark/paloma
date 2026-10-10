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

impl<'a> PromptBase<'a> {
    pub(crate) fn with_theme(theme: &'a dyn Theme) -> Self {
        PromptBase {
            theme: Some(theme),
            report: None,
        }
    }
}

/// Adds the shared `report` builder and `report_enabled` to a prompt that embeds a
/// [`PromptBase`] as `base`. `$group` is the prompt's field in
/// [`Options`](crate::Options); `$default` is a closure giving the built-in default.
macro_rules! report_methods {
    ($group:ident, $default:expr) => {
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
pub(crate) use report_methods;
