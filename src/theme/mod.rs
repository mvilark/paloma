//! How everything paloma draws looks: the [`Theme`] trait, design [`Tokens`] and the
//! built-in themes.
//!
//! Three layers, from cheapest to most powerful: change the [`Tokens`] (with
//! [`Theme::restyle`]), change one component's tokens (for example
//! [`PromptTokens`] or [`MessageTokens`]), or override a `format_*` hook. Every hook
//! takes a [`Context`] (the tokens, the stream and what its terminal can show) and a
//! view of what to draw, and its default body is public in [`defaults`].
use std::{borrow::Cow, fmt};

pub(crate) mod app;
mod colorful;
mod context;
pub mod defaults;
mod prompt;
pub(crate) mod render;
mod restyle;
mod simple;
mod tokens;

pub use crate::live::task::{TaskState, TaskStepView, TaskView};
pub use crate::output::{
    key_value::KeyValueView,
    message::{MessageView, Status},
    panel::PanelView,
    section::SectionView,
    table::TableView,
};
pub use colorful::ColorfulTheme;
pub use context::Context;
pub use prompt::{
    Answer, AnsweredView, ConfirmButtonsView, ConfirmPromptView, FlowSummaryView,
    FuzzySelectPromptView, InputPromptView, ItemView, LegendView, MessageSource, PromptKind,
    PromptMessageView, PromptView,
};
pub use restyle::Restyled;
pub use simple::SimpleTheme;
pub use tokens::{
    BlockKind, BlockLook, BlockStyle, BlockTokens, CursorShape, GlyphName, Glyphs, Layout,
    LiveKind, LiveLook, LiveStyle, LiveTokens, MessageLook, MessageStyle, MessageTokens, Palette,
    PromptLook, PromptStyle, PromptTokens, Role, Tokens,
};

/// Implements a theme for paloma.
pub trait Theme {
    /// The tokens this theme draws with. Defaults to [`Tokens::plain`].
    fn tokens(&self) -> Cow<'_, Tokens> {
        Cow::Owned(Tokens::plain())
    }

    /// The same theme drawn with other tokens: `f` gets this theme's tokens and returns
    /// the new ones. Works on any theme without implementing the trait.
    ///
    /// ```rust
    /// use paloma::theme::{ColorfulTheme, Theme};
    ///
    /// paloma::set_theme(ColorfulTheme.restyle(|t| t.glyphs(|g| g.warning("▲"))));
    /// ```
    fn restyle(self, f: impl FnOnce(Tokens) -> Tokens) -> Restyled<Self>
    where
        Self: Sized,
    {
        Restyled::new(self, f)
    }

    /// Formats one message line (see [`Message`](crate::Message)). The default is
    /// [`defaults::format_message`].
    fn format_message(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &MessageView,
    ) -> fmt::Result {
        defaults::format_message(f, ctx, view)
    }

    /// Formats a section title (see [`Section`](crate::Section)). The default is
    /// [`defaults::format_section`].
    fn format_section(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &SectionView,
    ) -> fmt::Result {
        defaults::format_section(f, ctx, view)
    }

    /// Formats key/value rows (see [`KeyValue`](crate::KeyValue)), one per line. The
    /// default is [`defaults::format_key_value`].
    fn format_key_value(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &KeyValueView,
    ) -> fmt::Result {
        defaults::format_key_value(f, ctx, view)
    }

    /// Formats a table (see [`Table`](crate::Table)). The default is
    /// [`defaults::format_table`].
    fn format_table(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &TableView) -> fmt::Result {
        defaults::format_table(f, ctx, view)
    }

    /// Formats a panel (see [`Panel`](crate::Panel)). The default is
    /// [`defaults::format_panel`].
    fn format_panel(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &PanelView) -> fmt::Result {
        defaults::format_panel(f, ctx, view)
    }

    /// Formats the line of a task (see [`Task`](crate::Task)): the spinner and label
    /// while it runs, the result line once it has finished. The default is
    /// [`defaults::format_task`].
    fn format_task(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &TaskView) -> fmt::Result {
        defaults::format_task(f, ctx, view)
    }

    /// Formats a nested step line under a running task. The default is
    /// [`defaults::format_task_step`].
    fn format_task_step(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &TaskStepView,
    ) -> fmt::Result {
        defaults::format_task_step(f, ctx, view)
    }

    /// Formats the question line of a list prompt (select, multi select, sort), with
    /// its paging counter. The default is [`defaults::format_prompt`].
    fn format_prompt(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &PromptView,
    ) -> fmt::Result {
        defaults::format_prompt(f, ctx, view)
    }

    /// Formats the line of an input or password prompt, up to where the typed text
    /// goes. It may span lines (the text then goes on the last one). The default is
    /// [`defaults::format_input_prompt`].
    fn format_input_prompt(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &InputPromptView,
    ) -> fmt::Result {
        defaults::format_input_prompt(f, ctx, view)
    }

    /// Formats the line of a confirm prompt; with buttons, only its question (the
    /// buttons go on the line under it, see
    /// [`format_confirm_buttons`](Self::format_confirm_buttons)). The default is
    /// [`defaults::format_confirm_prompt`].
    fn format_confirm_prompt(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &ConfirmPromptView,
    ) -> fmt::Result {
        defaults::format_confirm_prompt(f, ctx, view)
    }

    /// Formats the `Yes` / `No` buttons of a confirm prompt
    /// ([`Confirm::choices_as_buttons`](crate::Confirm::choices_as_buttons)). The
    /// default is [`defaults::format_confirm_buttons`].
    fn format_confirm_buttons(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &ConfirmButtonsView,
    ) -> fmt::Result {
        defaults::format_confirm_buttons(f, ctx, view)
    }

    /// Formats the search line of a fuzzy select, with its cursor. The default is
    /// [`defaults::format_fuzzy_select_prompt`].
    fn format_fuzzy_select_prompt(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &FuzzySelectPromptView,
    ) -> fmt::Result {
        defaults::format_fuzzy_select_prompt(f, ctx, view)
    }

    /// Formats one item of a list prompt (select, multi select, sort, fuzzy select).
    /// The default is [`defaults::format_item`].
    fn format_item(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &ItemView) -> fmt::Result {
        defaults::format_item(f, ctx, view)
    }

    /// Formats the line of keys under a prompt (see
    /// [`Select::legend`](crate::Select::legend)), drawn below the message row. The
    /// default is [`defaults::format_legend`].
    fn format_legend(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &LegendView,
    ) -> fmt::Result {
        defaults::format_legend(f, ctx, view)
    }

    /// Formats one line of the message row under a prompt (its description, what its
    /// `on_change` returned, or a validation error), drawn below everything else the
    /// prompt draws, one line per message and nested message. The default writes the
    /// `message_indent` token as spaces, then the line as this theme's
    /// [`format_message`](Self::format_message) draws it, so a theme that only
    /// overrides `format_message` draws its rows the same way;
    /// [`defaults::format_prompt_message`] does the same with the default message
    /// layout.
    fn format_prompt_message(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &PromptMessageView,
    ) -> fmt::Result {
        defaults::prompt_message_indent(f, ctx, view)?;
        let message = MessageView::new(view.status, view.text, view.depth);
        self.format_message(f, ctx, &message)
    }

    /// Formats the line a prompt leaves once answered (with [`Done::Collapse`](crate::Done::Collapse))
    /// or cancelled (the `cancelled_value` token).
    /// The default is [`defaults::format_answered`].
    fn format_answered(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &AnsweredView,
    ) -> fmt::Result {
        defaults::format_answered(f, ctx, view)
    }

    /// Formats the line a finished [`Flow`](crate::Flow) leaves with
    /// [`Done::Collapse`](crate::Done::Collapse): its title and how many steps ran. The
    /// default is [`defaults::format_flow_summary`].
    fn format_flow_summary(
        &self,
        f: &mut dyn fmt::Write,
        ctx: &Context,
        view: &FlowSummaryView,
    ) -> fmt::Result {
        defaults::format_flow_summary(f, ctx, view)
    }
}
