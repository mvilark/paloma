//! A theme with changed tokens, made by [`Theme::restyle`].
use std::{borrow::Cow, fmt};

use crate::theme::{
    AnsweredView, ConfirmButtonsView, ConfirmPromptView, Context, FuzzySelectPromptView,
    InputPromptView, ItemView, KeyValueView, LegendView, MessageView, PanelView, PromptMessageView,
    PromptView, SectionView, TableView, TaskStepView, TaskView, Theme, Tokens,
};

/// A theme drawn with other tokens. Every hook is the wrapped theme's own; only
/// [`tokens`](Theme::tokens) differs, so hooks that read tokens from their
/// [`Context`] use the new ones.
pub struct Restyled<T> {
    inner: T,
    tokens: Tokens,
}

impl<T: Theme> Restyled<T> {
    pub(crate) fn new(inner: T, f: impl FnOnce(Tokens) -> Tokens) -> Self {
        let tokens = f(inner.tokens().into_owned());
        Restyled { inner, tokens }
    }

    /// The wrapped theme.
    pub fn inner(&self) -> &T {
        &self.inner
    }
}

/// Forwards each listed hook to the wrapped theme.
macro_rules! forward {
    ($($(#[$meta:meta])* fn $name:ident(&self $(, $arg:ident: $ty:ty)*) -> $ret:ty;)*) => {
        $(
            $(#[$meta])*
            fn $name(&self $(, $arg: $ty)*) -> $ret {
                self.inner.$name($($arg),*)
            }
        )*
    };
}

impl<T: Theme> Theme for Restyled<T> {
    fn tokens(&self) -> Cow<'_, Tokens> {
        Cow::Borrowed(&self.tokens)
    }

    forward! {
        fn format_message(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &MessageView) -> fmt::Result;
        fn format_section(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &SectionView) -> fmt::Result;
        fn format_key_value(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &KeyValueView) -> fmt::Result;
        fn format_table(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &TableView) -> fmt::Result;
        fn format_panel(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &PanelView) -> fmt::Result;
        fn format_task(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &TaskView) -> fmt::Result;
        fn format_task_step(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &TaskStepView) -> fmt::Result;
        fn format_prompt(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &PromptView) -> fmt::Result;
        fn format_input_prompt(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &InputPromptView) -> fmt::Result;
        fn format_confirm_prompt(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &ConfirmPromptView) -> fmt::Result;
        fn format_confirm_buttons(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &ConfirmButtonsView) -> fmt::Result;
        fn format_fuzzy_select_prompt(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &FuzzySelectPromptView) -> fmt::Result;
        fn format_item(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &ItemView) -> fmt::Result;
        fn format_legend(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &LegendView) -> fmt::Result;
        fn format_prompt_message(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &PromptMessageView) -> fmt::Result;
        fn format_answered(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &AnsweredView) -> fmt::Result;
    }
}
