//! The app-wide theme set by [`set_theme`].
use std::sync::{Arc, RwLock};

use super::{SimpleTheme, Theme};

static APP_THEME: RwLock<Option<Arc<dyn Theme + Send + Sync>>> = RwLock::new(None);

/// Sets the theme for every prompt in the app.
///
/// A prompt given its own theme with `.theme(&t)` keeps it. Calling this again
/// replaces the app theme; prompts read it when they are shown.
///
/// ```rust,no_run
/// use paloma::{theme::ColorfulTheme, Confirm};
///
/// paloma::set_theme(ColorfulTheme);
/// let ok = Confirm::new().prompt("Continue?").interact().unwrap();
/// ```
pub fn set_theme<T: Theme + Send + Sync + 'static>(theme: T) {
    let mut guard = APP_THEME.write().unwrap_or_else(|e| e.into_inner());
    *guard = Some(Arc::new(theme));
}

/// Returns the app theme, or `SimpleTheme` if none was set.
pub(crate) fn current() -> Arc<dyn Theme + Send + Sync> {
    let guard = APP_THEME.read().unwrap_or_else(|e| e.into_inner());
    match &*guard {
        Some(theme) => Arc::clone(theme),
        None => Arc::new(SimpleTheme),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        terminal::Stream,
        theme::{ColorfulTheme, Context, ItemView, PromptKind, PromptView},
    };

    fn prompt_with(theme: &dyn Theme) -> String {
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let mut out = String::new();
        let view = PromptView::new(PromptKind::Select, "Name", None);
        theme.format_prompt(&mut out, &ctx, &view).unwrap();
        let view = ItemView::new(PromptKind::Select, "item", true, None, None);
        theme.format_item(&mut out, &ctx, &view).unwrap();
        out
    }

    // One test function: the app theme is shared by every test in the binary.
    #[test]
    fn app_theme_defaults_to_simple_then_follows_set_theme() {
        assert_eq!(prompt_with(&*current()), prompt_with(&SimpleTheme));

        let custom = ColorfulTheme;
        set_theme(ColorfulTheme);
        assert_eq!(prompt_with(&*current()), prompt_with(&custom));
        assert_ne!(prompt_with(&*current()), prompt_with(&SimpleTheme));
    }
}
