//! State and builder methods every prompt shares.
use crate::{output::message::Message, theme::Theme};

/// What all prompts have in common: their own theme, if any, whether to report, and
/// the description shown in their message row.
///
/// Each prompt embeds one as `base`. Settings left unset (`None`) fall through to
/// [`Options`](crate::Options).
#[derive(Clone)]
pub(crate) struct PromptBase<'a> {
    pub(crate) theme: Option<&'a dyn Theme>,
    pub(crate) report: Option<bool>,
    /// Shown in the message row while nothing newer applies.
    pub(crate) description: Option<Message<'static>>,
}

impl PromptBase<'_> {
    /// No theme of its own (the renderer then uses the app theme from
    /// [`set_theme`](crate::set_theme)) and nothing set.
    pub(crate) fn new() -> Self {
        PromptBase {
            theme: None,
            report: None,
            description: None,
        }
    }
}

/// Adds the builder methods every prompt shares (`theme`, `report`, `description`) and
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

        /// A line under the prompt, shown while nothing newer applies (see `on_change`
        /// and `validate`). Plain text is a description; any
        /// [`Message`](crate::Message) works too, such as
        /// `Message::hint("Ports below 1024 need root")`.
        pub fn description(mut self, message: impl crate::IntoMessage) -> Self {
            self.base.description = Some(message.into_message(crate::Status::Description));
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
