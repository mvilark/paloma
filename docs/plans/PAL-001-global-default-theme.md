# PAL-001: Global default theme

Roadmap item: [PAL-001 in ROADMAP.md](../../ROADMAP.md).

## Goal

Every prompt's `new()` / `default()` hard-codes `SimpleTheme` (for example `Self::with_theme(&SimpleTheme)` in `src/prompts/select.rs`), so an app that wants a consistent look has to call `with_theme(&theme)` on every prompt and keep a theme value in scope wherever it prompts (see `examples/input.rs`, `docs/USAGE.md`). This plan lets an app set its theme once, at startup, and have every prompt built with `new()` / `default()` use it. App developers benefit most: less boilerplate, and library code deep in an app can call `Confirm::new()` and still match the app's style.

## Proposed API / behavior

Two new public functions in `paloma::theme`:

```rust
/// Sets the theme used by prompts that were not given one with `with_theme()`.
/// Replaces any previously set default.
pub fn set_default<T: Theme + Send + Sync + 'static>(theme: T);

/// Goes back to `SimpleTheme` as the default.
pub fn reset_default();
```

App code:

```rust
use paloma::{theme::{self, ColorfulTheme}, Confirm, Input, Select};

fn main() -> paloma::Result<()> {
    theme::set_default(ColorfulTheme::default());

    let name: String = Input::new().with_prompt("Name").interact_text()?;
    let env = Select::new().with_prompt("Environment").items(&["staging", "prod"]).interact()?;
    if Confirm::new().with_prompt(format!("Deploy {name}?")).interact()? { /* ... */ }
    Ok(())
}
```

Decisions on the roadmap's open questions (all confirmed by spideyincrocs, see Status notes):

- **Global state vs. a context object.** A process-wide default. A context object (`let p = Prompter::new(theme); p.select()...`) still has to be passed around, which is the boilerplate this item removes, and apps can already get that pattern today by holding a theme and calling `with_theme(&theme)`. A context type can be a later roadmap item if someone needs per-scope themes.
- **Thread safety.** The default is stored in a `static DEFAULT_THEME: RwLock<Option<Arc<dyn Theme + Send + Sync>>>` (`RwLock::new` is `const`, fine for MSRV 1.85). That is why `set_default` requires `Send + Sync + 'static`. Both built-in themes satisfy it (checked: `ColorfulTheme` and `SimpleTheme` are `Send + Sync`). A custom theme holding `Rc`/`Cell` cannot be the global default but keeps working through `with_theme()`. Lock poisoning is ignored (`unwrap_or_else(PoisonError::into_inner)`) since the lock only guards a pointer swap.
- **Interaction with `with_theme()`.** An explicit `with_theme()` always wins. Prompts built with `new()` / `default()` look up the global default when they are interacted with, not when they are built, so it does not matter whether a prompt was constructed before or after `set_default` (this also covers `Clone`d prompts and prompts stored in structs). Each interaction takes one `Arc` clone and holds it until the interaction returns, so a concurrent `set_default` never changes a prompt that is already on screen.
- **Set once vs. replaceable.** Replaceable, with `reset_default()`. It costs nothing extra with the `Arc` design and lets apps switch themes (for example a `--no-color` flag parsed after setup) and lets tests restore the default.

Backwards compatibility: purely additive. Apps that never call `set_default` render exactly as today. The changed prompt field (`theme`) is private, and no new external types enter the public API, so cargo-semver-checks and cargo-check-external-types should stay green. This is a minor-version change.

## Implementation steps

1. **Global default** (new private module `src/theme/default.rs`, declared and re-exported from `src/theme/mod.rs`):
   - `static DEFAULT_THEME: RwLock<Option<Arc<dyn Theme + Send + Sync>>> = RwLock::new(None);`
   - `pub fn set_default<T: Theme + Send + Sync + 'static>(theme: T)` stores `Some(Arc::new(theme))`; `pub fn reset_default()` stores `None`.
   - Crate-private resolver so prompts don't repeat the lookup:
     ```rust
     pub(crate) enum ResolvedTheme<'a> {
         Explicit(&'a dyn Theme),
         Default(Arc<dyn Theme + Send + Sync>),
     }
     impl Deref for ResolvedTheme<'_> { type Target = dyn Theme; /* ... */ }
     pub(crate) fn resolve(theme: Option<&dyn Theme>) -> ResolvedTheme<'_>;
     ```
     `resolve(None)` returns the stored default, or `Arc::new(SimpleTheme)` when none is set.
   - Rustdoc on both public functions with an example and the `Send + Sync` note.
2. **Prompts** (`confirm.rs`, `input.rs`, `select.rs`, `multi_select.rs`, `sort.rs`, `fuzzy_select.rs`, `password.rs` in `src/prompts/`):
   - Field `theme: &'a dyn Theme` becomes `theme: Option<&'a dyn Theme>`.
   - `new()` builds the prompt with `theme: None` instead of `with_theme(&SimpleTheme)`. `with_theme` is the constructor that sets every field today, so move the field defaults into a private `fn with_theme_opt(theme: Option<&'a dyn Theme>) -> Self` that both `new()` and `with_theme()` call.
   - `with_theme(theme)` stores `Some(theme)`.
   - At each `TermThemeRenderer::new(term, self.theme)` call (8 sites: one per prompt, two in `input.rs`) do `let theme = theme::resolve(self.theme);` then `TermThemeRenderer::new(term, &*theme)`. `TermThemeRenderer` itself does not change.
   - Update the `new()` doc comments from "with default theme" to say it uses the theme set by `set_default`, or `SimpleTheme`.
3. **Tests** (unit tests in `src/theme/default.rs`, matching the repo's `#[cfg(test)]` style):
   - One `#[test]` function owns every set/reset assertion, because tests in a binary run on parallel threads and share the global. No other test touches the global (prompt unit tests never assert on rendering, so they are unaffected).
   - Cover: unset, `resolve(None)` formats like `SimpleTheme`; after `set_default(MarkerTheme)`, `resolve(None)` uses `MarkerTheme`; `resolve(Some(&other))` still returns `other`; after `reset_default()`, back to `SimpleTheme`. Compare output of a cheap method such as `format_prompt` into a `String`.
   - Compile-time check that `ColorfulTheme` and `SimpleTheme` are `Send + Sync`, so a future field cannot silently break `set_default(ColorfulTheme::default())`.
   - Existing `test_clone` tests keep passing with the `Option` field.
4. **Docs and examples**:
   - New `examples/default_theme.rs`: set `ColorfulTheme` once, then run `Input::new()`, `Select::new()`, `Confirm::new()`.
   - `docs/USAGE.md`: in the quick-start and the "Themes" section, show `set_default` and state the precedence (`with_theme()` > global default > `SimpleTheme`) and the `Send + Sync` requirement. Update the line that says `new()` "uses `SimpleTheme`".
   - `DEVELOPMENT.md`: in the prompt conventions, note that `new()` leaves `theme` as `None` and rendering goes through `theme::resolve`; new prompts must follow that.
   - Crate-level docs in `src/lib.rs`: one line under Crate Contents.
5. **Checks before the PR**: `cargo test --locked --all-features`, `cargo lint`, `cargo format-check`, `cargo check --examples --all-features`, the `cargo doc` command from `DEVELOPMENT.md`, and an MSRV check (`cargo +1.85 check --lib --all-features`) if the toolchain is available.

## Open questions / risks

- **Hidden global state.** Library crates that call paloma will pick up whatever theme the host app set. That is the intent, but it is a behavior change for anyone relying on `new()` always being plain; it only happens if the app opts in, so the risk is low. Document it on `new()`.
- **Public getter.** Decided: no. The resolver stays crate-private; a public `default_theme()` getter can be added later without breaking anything if someone needs to wrap the current default.
- **Per-prompt cost.** One `RwLock` read and one `Arc` clone per interaction (plus a small allocation when no default is set). Negligible next to terminal I/O.
- **`ColorfulTheme` gaining a non-`Sync` field** would break `set_default(ColorfulTheme::default())`; the compile-time test in step 3 catches it.

## Status notes

- 2026-10-10: Plan written; item moved to Planned.
- 2026-10-10: Naming decided by spideyincrocs: `paloma::theme::set_default(t)` and `reset_default()`.
- 2026-10-10: Global default (not a context object) confirmed by spideyincrocs.
- 2026-10-10: Replaceable default with `reset_default()` (not set once) confirmed by spideyincrocs.
- 2026-10-10: Look up the default when a prompt is shown (not when it is built) confirmed by spideyincrocs.
- 2026-10-10: Keep the default lookup internal, no public getter, confirmed by spideyincrocs. All design decisions settled; plan ready for review.
