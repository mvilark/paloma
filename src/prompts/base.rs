//! State and builder methods every prompt shares.
use crate::{options::Done, output::message::Message, theme::Theme};

/// What all prompts have in common: their own theme, if any, what they leave when
/// answered, whether they show a legend, and the description shown in their message
/// row.
///
/// Each prompt embeds one as `base`. Settings left unset (`None`) fall through to
/// [`Options`](crate::Options).
#[derive(Clone)]
pub(crate) struct PromptBase<'a> {
    pub(crate) theme: Option<&'a dyn Theme>,
    pub(crate) done: Option<Done>,
    pub(crate) legend: Option<bool>,
    /// Shown in the message row while nothing newer applies.
    pub(crate) description: Option<Message<'static>>,
}

impl PromptBase<'_> {
    /// No theme of its own (the renderer then uses the app theme from
    /// [`set_theme`](crate::set_theme)) and nothing set.
    pub(crate) fn new() -> Self {
        PromptBase {
            theme: None,
            done: None,
            legend: None,
            description: None,
        }
    }
}

/// Adds the builder methods every prompt shares (`theme`, `done`, `legend`,
/// `description`) and `done_mode` / `legend_enabled` (the settings as resolved against
/// the app options) to a prompt that embeds a [`PromptBase`] as `base`. Use it inside
/// an `impl<'a> Prompt<'a>` block. `$group` is the prompt's field in
/// [`Options`](crate::Options).
macro_rules! base_methods {
    ($group:ident) => {
        /// Draws this prompt with `theme` instead of the app theme set with
        /// [`set_theme`](crate::set_theme).
        pub fn theme(mut self, theme: &'a dyn crate::theme::Theme) -> Self {
            self.base.theme = Some(theme);
            self
        }

        /// What the prompt leaves on the screen once answered: one line with the answer
        /// ([`Done::Collapse`](crate::Done::Collapse), the default), the prompt as it
        /// was ([`Done::Keep`](crate::Done::Keep)) or nothing
        /// ([`Done::Erase`](crate::Done::Erase)). A prompt left with `Esc` always
        /// leaves nothing.
        ///
        /// Left unset, the app option applies ([`Options`](crate::Options)).
        pub fn done(mut self, val: crate::Done) -> Self {
            self.base.done = Some(val);
            self
        }

        /// Whether to show a line with the prompt's keys under it (drawn by the
        /// theme's `format_legend`, with the words of the `legend` prompt tokens).
        ///
        /// Left unset, the app option applies; the built-in default is off.
        pub fn legend(mut self, val: bool) -> Self {
            self.base.legend = Some(val);
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

        /// What the prompt leaves once answered: its own value, the group's, the
        /// `prompts` family's, then `Collapse`.
        fn done_mode(&self, options: &crate::options::Options) -> crate::Done {
            options.done(self.base.done, options.$group.done)
        }

        /// Whether the prompt shows its legend.
        fn legend_enabled(&self, options: &crate::options::Options) -> bool {
            options.legend(self.base.legend, options.$group.legend)
        }
    };
}
pub(crate) use base_methods;
