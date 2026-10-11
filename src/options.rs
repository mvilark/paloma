//! App-wide behavior options set by [`set_options`].
//!
//! Themes decide how things look; options decide how they work. Settings are grouped
//! by component family (`prompts`, `messages`, `blocks`, `live`), with one group per
//! member underneath (`confirm`, `select`, …, `error`, `warning`, …, `task`):
//!
//! ```rust,no_run
//! use paloma::{terminal::Stream, Options};
//!
//! paloma::set_options(
//!     Options::new()
//!         .prompts(|p| p.report(true))
//!         .confirm(|c| c.choices_as_buttons(true).on_shortcut(false))
//!         .password(|p| p.mask(true).result_len(8))
//!         .select(|s| s.report(false))
//!         .messages(|m| m.stream(Stream::Stderr))
//!         .success(|s| s.stream(Stream::Stdout)),
//! );
//! ```
//!
//! A setting is resolved in this order: the component's own builder method, its member
//! group (`select`, `error`), its family group (`prompts`, `messages`, …), then the
//! built-in default.
use std::sync::RwLock;

use crate::{
    output::message::Status,
    settings::settings,
    terminal::Stream,
    theme::{BlockKind, LiveKind},
};

static APP_OPTIONS: RwLock<Option<Options>> = RwLock::new(None);

/// Sets the behavior options for every component in the app.
///
/// Calling this again replaces the options; components read them when they are shown.
pub fn set_options(options: Options) {
    let mut guard = APP_OPTIONS.write().unwrap_or_else(|e| e.into_inner());
    *guard = Some(options);
}

/// Returns the app options, or the defaults if none were set.
pub(crate) fn current() -> Options {
    let guard = APP_OPTIONS.read().unwrap_or_else(|e| e.into_inner());
    guard.clone().unwrap_or_default()
}

settings! {
    /// Settings for the whole app.
    pub struct GeneralOptions {
        /// Whether a prompt prints its one-line result after it is answered.
        #[deprecated(note = "use `Options::prompts(|p| p.report(..))`")]
        report: bool,
    }
}

settings! {
    /// Settings shared by prompts: the `prompts` family group, and the group of
    /// `Input`, `Select`, `MultiSelect`, `Sort` and `FuzzySelect`.
    pub struct PromptOptions {
        /// Whether a prompt prints its one-line result after it is answered.
        report: bool,
    }
}

settings! {
    /// Settings for [`Confirm`](crate::Confirm).
    pub struct ConfirmOptions {
        /// Whether the prompt prints its one-line result after it is answered.
        report: bool,
        /// `Yes` / `No` buttons (`true`) or the classic `(y/n)` text (`false`, the
        /// default). See [`Confirm::choices_as_buttons`](crate::Confirm::choices_as_buttons).
        choices_as_buttons: bool,
        /// Whether `y` / `n` answer at once (`true`, the default) or only select the
        /// choice so the user hits `Enter` to submit (`false`). Works with and without
        /// buttons.
        on_shortcut: bool,
    }
}

settings! {
    /// Settings for [`Password`](crate::Password).
    pub struct PasswordOptions {
        /// Whether the prompt prints its one-line result after it is answered.
        report: bool,
        /// Whether typed characters are echoed as mask characters (`true`) or not at
        /// all (`false`, the default). The theme picks the character with
        /// [`password_mask_glyph`](crate::theme::Theme::password_mask_glyph).
        mask: bool,
        /// How many mask characters the one-line result shows, whatever the password's
        /// length. The default is 8.
        result_len: usize,
    }
}

settings! {
    /// Settings for [`Message`](crate::Message): the `messages` family group and the
    /// group of each status.
    pub struct MessageOptions {
        /// Where messages are printed. The default is stderr.
        stream: Stream,
    }
}

settings! {
    /// Settings for output blocks ([`Section`](crate::Section),
    /// [`KeyValue`](crate::KeyValue), [`Table`](crate::Table), [`Panel`](crate::Panel)):
    /// the `blocks` family group and the group of each block.
    pub struct BlockOptions {
        /// Where blocks are printed. The default is stderr for sections and panels and
        /// stdout for key/value rows and tables, which are usually results.
        stream: Stream,
    }
}

settings! {
    /// Settings for live components ([`Task`](crate::Task)): the `live` family group
    /// and the group of each component (`task`).
    pub struct LiveOptions {
        /// Whether live components redraw (spinners turn). Default: on when the stream
        /// is a terminal and `CI` is not set. Never on when the stream is not a terminal.
        motion: bool,
        /// Milliseconds between frames. Default 80.
        frame_ms: u64,
        /// Where live components draw. Default stderr.
        stream: Stream,
    }
}

/// App-wide behavior options. See the [module docs](self).
#[derive(Clone, Default, Debug)]
pub struct Options {
    pub(crate) general: GeneralOptions,
    pub(crate) prompts: PromptOptions,
    pub(crate) confirm: ConfirmOptions,
    pub(crate) password: PasswordOptions,
    pub(crate) input: PromptOptions,
    pub(crate) select: PromptOptions,
    pub(crate) multi_select: PromptOptions,
    pub(crate) sort: PromptOptions,
    pub(crate) fuzzy_select: PromptOptions,
    pub(crate) messages: MessageOptions,
    pub(crate) description: MessageOptions,
    pub(crate) hint: MessageOptions,
    pub(crate) info: MessageOptions,
    pub(crate) success: MessageOptions,
    pub(crate) warning: MessageOptions,
    pub(crate) error: MessageOptions,
    pub(crate) blocks: BlockOptions,
    pub(crate) section: BlockOptions,
    pub(crate) key_value: BlockOptions,
    pub(crate) table: BlockOptions,
    pub(crate) panel: BlockOptions,
    pub(crate) live: LiveOptions,
    pub(crate) task: LiveOptions,
}

macro_rules! groups {
    ($($(#[$meta:meta])* $group:ident: $ty:ty),* $(,)?) => {
        impl Options {
            $(
                $(#[$meta])*
                pub fn $group(mut self, f: impl FnOnce($ty) -> $ty) -> Self {
                    self.$group = f(self.$group);
                    self
                }
            )*
        }
    };
}

groups! {
    /// Settings for the whole app.
    general: GeneralOptions,
    /// Settings shared by every prompt.
    prompts: PromptOptions,
    /// Settings for [`Confirm`](crate::Confirm).
    confirm: ConfirmOptions,
    /// Settings for [`Password`](crate::Password).
    password: PasswordOptions,
    /// Settings for [`Input`](crate::Input).
    input: PromptOptions,
    /// Settings for [`Select`](crate::Select).
    select: PromptOptions,
    /// Settings for [`MultiSelect`](crate::MultiSelect).
    multi_select: PromptOptions,
    /// Settings for [`Sort`](crate::Sort).
    sort: PromptOptions,
    /// Settings for `FuzzySelect` (feature `fuzzy-select`).
    fuzzy_select: PromptOptions,
    /// Settings shared by every [`Message`](crate::Message).
    messages: MessageOptions,
    /// Settings for description messages.
    description: MessageOptions,
    /// Settings for hint messages.
    hint: MessageOptions,
    /// Settings for info messages.
    info: MessageOptions,
    /// Settings for success messages.
    success: MessageOptions,
    /// Settings for warnings.
    warning: MessageOptions,
    /// Settings for error messages.
    error: MessageOptions,
    /// Settings shared by every output block.
    blocks: BlockOptions,
    /// Settings for [`Section`](crate::Section).
    section: BlockOptions,
    /// Settings for [`KeyValue`](crate::KeyValue).
    key_value: BlockOptions,
    /// Settings for [`Table`](crate::Table).
    table: BlockOptions,
    /// Settings for [`Panel`](crate::Panel).
    panel: BlockOptions,
    /// Settings shared by every live component.
    live: LiveOptions,
    /// Settings for [`Task`](crate::Task).
    task: LiveOptions,
}

impl Options {
    /// Creates options with nothing set, so every component uses its built-in defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves `report`: the prompt's own value, its group, `prompts`, then the
    /// deprecated `general` value.
    pub(crate) fn report(&self, own: Option<bool>, group: Option<bool>) -> Option<bool> {
        own.or(group)
            .or(self.prompts.report)
            .or(self.general.report)
    }

    /// The group of one output block.
    pub(crate) fn block(&self, kind: BlockKind) -> &BlockOptions {
        match kind {
            BlockKind::Section => &self.section,
            BlockKind::KeyValue => &self.key_value,
            BlockKind::Table => &self.table,
            BlockKind::Panel => &self.panel,
        }
    }

    /// The group of one live component.
    pub(crate) fn live_member(&self, kind: LiveKind) -> &LiveOptions {
        match kind {
            LiveKind::Task => &self.task,
        }
    }

    /// The group of one message status.
    pub(crate) fn status(&self, status: Status) -> &MessageOptions {
        match status {
            Status::Description => &self.description,
            Status::Hint => &self.hint,
            Status::Info => &self.info,
            Status::Success => &self.success,
            Status::Warning => &self.warning,
            Status::Error => &self.error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_resolves_prompt_then_group_then_family_then_general() {
        let o = Options::new()
            .prompts(|p| p.report(false))
            .select(|s| s.report(true));
        assert_eq!(o.report(None, o.select.report), Some(true));
        assert_eq!(o.report(None, o.input.report), Some(false));
        assert_eq!(o.report(Some(true), o.input.report), Some(true));
        assert_eq!(Options::new().report(None, None), None);

        #[allow(deprecated)]
        let o = Options::new().general(|g| g.report(false));
        assert_eq!(o.report(None, None), Some(false));
    }

    #[test]
    fn message_stream_resolves_status_then_family() {
        let o = Options::new()
            .messages(|m| m.stream(Stream::Stdout))
            .error(|e| e.stream(Stream::Stderr));
        let stream = |s: Status| o.status(s).stream.or(o.messages.stream);
        assert_eq!(stream(Status::Error), Some(Stream::Stderr));
        assert_eq!(stream(Status::Info), Some(Stream::Stdout));
    }

    #[test]
    fn live_settings_resolve_task_then_live() {
        let o = Options::new()
            .live(|l| l.frame_ms(120).motion(false))
            .task(|t| t.frame_ms(40));
        let member = o.live_member(LiveKind::Task);
        assert_eq!(member.frame_ms.or(o.live.frame_ms), Some(40));
        assert_eq!(member.motion.or(o.live.motion), Some(false));
        assert_eq!(member.stream.or(o.live.stream), None);
    }
}
