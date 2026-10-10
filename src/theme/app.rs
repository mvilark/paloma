//! The app-wide theme set by [`set_theme`].
use std::sync::{Arc, RwLock};

use super::{SimpleTheme, Theme};

static APP_THEME: RwLock<Option<Arc<dyn Theme + Send + Sync>>> = RwLock::new(None);

/// Sets the theme for every prompt in the app.
///
/// A prompt created with `with_theme` keeps its own theme. Calling this again
/// replaces the app theme; prompts read it when they are shown.
///
/// ```rust,no_run
/// use paloma::{theme::ColorfulTheme, Confirm};
///
/// paloma::set_theme(ColorfulTheme::default());
/// let ok = Confirm::new().with_prompt("Continue?").interact().unwrap();
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
    use crate::theme::ColorfulTheme;

    fn prompt_with(theme: &dyn Theme) -> String {
        let mut out = String::new();
        theme.format_prompt(&mut out, "Name").unwrap();
        theme
            .format_select_prompt_item(&mut out, "item", true)
            .unwrap();
        out
    }

    // One test function: the app theme is shared by every test in the binary.
    #[test]
    fn app_theme_defaults_to_simple_then_follows_set_theme() {
        assert_eq!(prompt_with(&*current()), prompt_with(&SimpleTheme));

        let custom = ColorfulTheme::default();
        set_theme(ColorfulTheme::default());
        assert_eq!(prompt_with(&*current()), prompt_with(&custom));
        assert_ne!(prompt_with(&*current()), prompt_with(&SimpleTheme));
    }
}
