//! A theme with changed tokens, made by [`Theme::restyle`].
use std::{borrow::Cow, fmt};

#[cfg(feature = "fuzzy-select")]
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::theme::{
    Context, KeyValueView, MessageView, PanelView, SectionView, TableView, TaskStepView, TaskView,
    Theme, Tokens,
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
        fn format_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result;
        fn format_error(&self, f: &mut dyn fmt::Write, err: &str) -> fmt::Result;
        fn format_confirm_prompt(&self, f: &mut dyn fmt::Write, prompt: &str, default: Option<bool>) -> fmt::Result;
        fn format_confirm_prompt_no_hint(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result;
        fn format_confirm_button(&self, f: &mut dyn fmt::Write, yes: bool, focused: bool) -> fmt::Result;
        fn format_confirm_buttons(&self, f: &mut dyn fmt::Write, yes_focused: bool) -> fmt::Result;
        fn format_confirm_prompt_selection(&self, f: &mut dyn fmt::Write, prompt: &str, selection: Option<bool>) -> fmt::Result;
        fn format_input_prompt(&self, f: &mut dyn fmt::Write, prompt: &str, default: Option<&str>) -> fmt::Result;
        fn format_input_prompt_selection(&self, f: &mut dyn fmt::Write, prompt: &str, sel: &str) -> fmt::Result;
        #[cfg(feature = "password")]
        fn format_password_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result;
        #[cfg(feature = "password")]
        fn format_password_prompt_selection(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result;
        #[cfg(feature = "password")]
        fn password_mask_glyph(&self) -> char;
        #[cfg(feature = "password")]
        fn format_password_prompt_selection_masked(&self, f: &mut dyn fmt::Write, prompt: &str, mask_len: usize) -> fmt::Result;
        fn format_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result;
        fn format_select_prompt_selection(&self, f: &mut dyn fmt::Write, prompt: &str, sel: &str) -> fmt::Result;
        fn format_multi_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result;
        fn format_sort_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result;
        fn format_multi_select_prompt_selection(&self, f: &mut dyn fmt::Write, prompt: &str, selections: &[&str]) -> fmt::Result;
        fn format_sort_prompt_selection(&self, f: &mut dyn fmt::Write, prompt: &str, selections: &[&str]) -> fmt::Result;
        fn format_select_prompt_item(&self, f: &mut dyn fmt::Write, text: &str, active: bool) -> fmt::Result;
        fn format_multi_select_prompt_item(&self, f: &mut dyn fmt::Write, text: &str, checked: bool, active: bool) -> fmt::Result;
        fn format_sort_prompt_item(&self, f: &mut dyn fmt::Write, text: &str, picked: bool, active: bool) -> fmt::Result;
        #[cfg(feature = "fuzzy-select")]
        fn format_fuzzy_select_prompt_item(&self, f: &mut dyn fmt::Write, text: &str, active: bool, highlight_matches: bool, matcher: &SkimMatcherV2, search_term: &str) -> fmt::Result;
        #[cfg(feature = "fuzzy-select")]
        fn format_fuzzy_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str, search_term: &str, bytes_pos: usize) -> fmt::Result;
    }
}
