# PAL-001: App-wide theme with `set_theme`

Roadmap item: [PAL-001 in ROADMAP.md](../../ROADMAP.md).

## Goal

Today every prompt built with `new()` uses `SimpleTheme`, so an app that wants another look calls `with_theme(&theme)` on every prompt. This plan adds one call that sets the theme for the whole app.

## Proposed API / behavior

One new public function at the crate root:

```rust
/// Sets the theme for every prompt in the app.
pub fn set_theme<T: Theme + Send + Sync + 'static>(theme: T);
```

```rust
use paloma::{theme::ColorfulTheme, Confirm, Input};

fn main() -> paloma::Result<()> {
    paloma::set_theme(ColorfulTheme::default());

    let name: String = Input::new().with_prompt("Name").interact_text()?; // ColorfulTheme
    let ok = Confirm::with_theme(&DangerTheme)                            // DangerTheme, this prompt only
        .with_prompt(format!("Delete {name}?"))
        .interact()?;
    Ok(())
}
```

- `set_theme(t)` makes `t` the theme for every prompt in the app.
- `with_theme(&t)` on a prompt overrides it for that prompt only, exactly as it works today.
- Before `set_theme` is called, prompts use `SimpleTheme`, as today.
- Calling `set_theme` again replaces the theme. Prompts read it when they are shown, so it does not matter whether a prompt was built before or after the call.
- The theme must be `Send + Sync` because it is shared across the whole app. `SimpleTheme` and `ColorfulTheme` both are.

This is purely additive: apps that never call `set_theme` see no change.

## Implementation steps

1. **Storage** in a new private module `src/theme/app.rs`, re-exported from `src/lib.rs` as `paloma::set_theme`:
   - `static APP_THEME: RwLock<Option<Arc<dyn Theme + Send + Sync>>> = RwLock::new(None);`
   - `pub fn set_theme<T: Theme + Send + Sync + 'static>(theme: T)` stores `Some(Arc::new(theme))`.
   - A crate-private `fn current() -> Arc<dyn Theme + Send + Sync>` returns the stored theme, or `SimpleTheme` if none is set.
2. **Prompts** (`confirm.rs`, `input.rs`, `select.rs`, `multi_select.rs`, `sort.rs`, `fuzzy_select.rs`, `password.rs` in `src/prompts/`):
   - The private field `theme: &'a dyn Theme` becomes `Option<&'a dyn Theme>`. `new()` sets `None`; `with_theme(&t)` sets `Some(&t)`.
   - Where a prompt starts rendering (`TermThemeRenderer::new`, 8 call sites), use the prompt's own theme if it has one, otherwise `theme::app::current()`.
3. **Tests**: one unit test in `src/theme/app.rs` that checks `current()` formats like `SimpleTheme` before `set_theme`, then like the theme passed to `set_theme` (compare `format_prompt` output). It is a single test function because the app theme is shared by every test in the binary. Prompts cannot render without a real terminal, so the `with_theme` override is covered by keeping the selection logic in one small helper next to `current()` and testing that helper too.
4. **Docs and examples**: add `examples/set_theme.rs`; in `docs/USAGE.md`, show `paloma::set_theme` in the quick start and the Themes section; in `DEVELOPMENT.md`, note that new prompts must follow the same `Option` field pattern.
5. **Checks**: the commands in `DEVELOPMENT.md` (test, lint, format, examples, docs).

## Open questions / risks

- A custom theme that is not `Send + Sync` (one holding an `Rc` or `Cell`) cannot be passed to `set_theme`. It still works with `with_theme`.

## Status notes

- 2026-10-10: Plan written after the earlier plan (PR #3) was dropped for this simpler design.
- 2026-10-10: spideyincrocs chose to keep `Select::with_theme(&t)` as the per-prompt override (no chained method, nothing breaks).
