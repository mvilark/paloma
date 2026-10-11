# Development

No CONTRIBUTING.md exists yet. This file covers contributor workflow; for API usage see [docs/USAGE.md](docs/USAGE.md). The crate is named `paloma` (forked from [dialoguer](https://github.com/console-rs/dialoguer)).

## Layout
- `src/lib.rs` - public re-exports, module declarations and feature gates (`#![deny(clippy::all, missing_docs)]`). Nothing else lives there.
- `src/prompts/` - one file per prompt: `confirm`, `input`, `select`, `multi_select`, `sort`, `password`, `fuzzy_select`; `base.rs` holds `PromptBase` (state every prompt shares) and the `report_methods!` macro.
- `src/output/` - components that print once: `message.rs` (`Message`, `Status`), `block.rs` (`BlockBase`, `block_methods!` and the shared `emit` printing helper), `section.rs`, `key_value.rs`, `table.rs`, `panel.rs`, and `text.rs` (`link`, `gradient`). Each component module also defines the view its theme hook receives.
- `src/live/` - components that redraw while work runs: `engine.rs` (the one live region: redraw thread, hidden cursor, pause for prompts, `restore`) and `task.rs` (`Task`, `RunningTask`, and the views their theme hooks receive).
- `src/options.rs` - app-wide behaviour (`set_options`, `Options`, family groups `prompts` / `messages` / `blocks` / `live` and member groups). `src/settings.rs` - the crate-private `settings!` macro every option group and component-token group is declared with.
- `src/terminal.rs` - `Stream`, `Capabilities`, `capabilities()`, `restore()`.
- `src/theme/` - `Theme` trait (`mod.rs`), `SimpleTheme`, `ColorfulTheme`, `tokens.rs` (`Tokens`, `Palette`, `Glyphs`, `Layout`, component tokens), `context.rs` (`Context` passed to new-style hooks), `defaults.rs` (public default bodies of those hooks), `restyle.rs` (`Restyled`, which must forward every `Theme` method), `app.rs` (app-wide theme, `set_theme`) and `render.rs` (internal `TermThemeRenderer` that prompts draw through).
- `src/validate.rs`, `history.rs`, `completion.rs`, `edit.rs` (`Editor`), `paging.rs`, `error.rs`.
- `examples/` - one runnable example per feature (`cargo run --example wizard`). `examples/ultraviolet/` is a larger fake VPN CLI themed with the Ultraviolet design system (`cargo run --example ultraviolet --all-features -- <command>`); its `ui.rs` draws the non-prompt output and its gaps are listed in [docs/plans/PAL-002-ultraviolet-example.md](docs/plans/PAL-002-ultraviolet-example.md).
- `tests/` - integration tests: golden output of every `Theme` hook for the built-in themes (`themes.rs`) and transcripts of the Ultraviolet example's non-interactive commands (`ultraviolet.rs`, Unix only). Expected output lives in `tests/snapshots/`.
- `.cargo/config.toml` - aliases used by CI. `deny.toml` - cargo-deny config. `CHANGELOG-OLD.md` - changelog up to 0.11.0.

## Features
Defaults: `editor` (tempfile), `password` (zeroize). Optional: `fuzzy-select`, `history`, `completion`. Examples for gated features declare `required-features` in `Cargo.toml`.

## Build, test, lint
```
cargo test --locked --all-features
cargo lint            # clippy --locked --all-targets --all-features -- -D warnings
cargo format-check    # cargo fmt --check  (cargo format to fix; default rustfmt, no config)
cargo check --examples --all-features
RUSTDOCFLAGS=-Dwarnings cargo doc --locked --all-features --no-deps --document-private-items
```
Unit tests sit in a `#[cfg(test)]` module at the bottom of the file they test (prompts, `options.rs`, `theme/app.rs`, `theme/render.rs`, and the Ultraviolet example's `theme.rs`, which `cargo test` runs because its `[[example]]` sets `test = true`). Coverage alias: `cargo test-cover` (needs cargo-llvm-cov).

The golden tests in `tests/` fail on any change to what the built-in themes or the Ultraviolet example print. When a change is intended, run `UPDATE_SNAPSHOTS=1 cargo test --all-features`, review the diff of `tests/snapshots/`, and say in the PR why the output changed. Tests that set app-wide state (`set_theme`, `set_options`, colour support) share one test function per binary, because that state is global.

Prompts need a real terminal. To check one by hand without a terminal of your own, drive it through a pseudo-terminal (for example Python's `pty` with the `pyte` emulator) and look at the final screen, as PAL-005 did.

CI (`.github/workflows/ci.yml`) also runs: tests on Linux/macOS/Windows, MSRV check (Rust 1.85, `cargo check --lib --all-features`), cargo-check-external-types (only `console` and `fuzzy_matcher::skim::SkimMatcherV2` may appear in the public API), and cargo-deny audit. The cargo-semver-checks job is switched off (`if: false`) until the first release on crates.io, since there is nothing to compare against yet. GitHub Actions stays off until the crate is published on crates.io, so nothing runs these checks for you: run the commands above (and the MSRV check, `cargo +1.85.0 check --locked --lib --all-features`) before every push and list them in the PR.

## Conventions
- **Not opinionated, sensible defaults.** paloma prescribes no look or behaviour. Every new behaviour is a default with a setting (builder method and `Options`), every new look goes through the theme, and nothing should ever force an app to write its own UI code or work below paloma's interface. At the same time every component must work out of the box with minimal code. A design system's rules (such as Ultraviolet's) belong in its theme and options, never in paloma's defaults or code.
- **Plans are written for an agent without context.** Every `docs/plans/PAL-NNN-*.md` must let a separate agent that has only studied the repo and its Markdown files implement it alone, and names the executor (model and effort) chosen for it. See [docs/plans/TEMPLATE.md](docs/plans/TEMPLATE.md).
- MSRV 1.85; do not use newer language/std features.
- Builder style: `new()` is the only constructor; setters are named after what they set (`prompt`, `theme`, `default`, `validate`), never with a `with_` prefix or `_with` suffix, and take `mut self -> Self`; prompts end with the `interact` / `interact_opt` / `interact_on` / `interact_on_opt` terminals returning `crate::Result`. Prompts derive/impl `Clone` and `Default` where sensible.
- Every prompt embeds a `PromptBase` as `base` (`PromptBase::new()` in its `new()`) and calls `base_methods!` inside an `impl<'a> Prompt<'a>` block, which adds the setters all prompts share (`theme`, `report`). Without `.theme(&t)` the renderer uses the app theme from `set_theme`. New prompts follow the same pattern; a setter that every prompt should have goes in `base_methods!`, not in each prompt.
- Themes decide how things look; `Options` decide how they work. A behaviour setting is an `Option` resolved in this order: the prompt's builder method, its group in `Options`, `general`, the built-in default. Read the app options once, when the prompt is shown (`crate::options::current()`), not inside loops.
- Everything paloma draws goes through a `Theme` method (for prompts, via `render.rs`); components never write styled text or layout themselves.
- Public items need doc comments (`missing_docs` is denied); docs build with warnings denied.
- Gate optional code with `#[cfg(feature = "...")]` and update both `lib.rs` and docs.rs metadata (`all-features = true`).
- Avoid leaking new third-party types in the public API (external-types check).

## Adding a prompt option
1. Add a field to the prompt struct and its `new()` constructor in `src/prompts/<prompt>.rs`. If an app should be able to set it for every prompt of that kind, make the field an `Option` and add the same setting to the prompt's group in `src/options.rs` (and to `GeneralOptions` if it applies to all prompts; shared settings like `report` go in `PromptBase`).
2. Add a documented builder method following the existing pattern.
3. Use it in `interact_on*`; if it changes rendering, route it through the theme (below).
4. Add a unit test in the file's `tests` module, update an example if useful, and mention it in `docs/USAGE.md`.

## Adding a new prompt type
Create `src/prompts/<name>.rs` with a `base: PromptBase` field and `base_methods!`, register it in `src/prompts/mod.rs`, add its group to `Options`, re-export from `src/lib.rs` (feature-gated if it adds a dependency, plus a `[[example]]` entry with `required-features`), add `format_<name>_*` methods with defaults to `Theme`, extend `tests/themes.rs` with the new hooks, and add an example.

## Adding or changing theme options
- New styling field: add it to `ColorfulTheme` (`src/theme/colorful.rs`) and its `Default` impl; use it in the relevant `format_*` override. Adding a public field is a semver break for struct-literal users without `..Default::default()`, so check cargo-semver-checks.
- New rendering hook: add a `format_*` method with a default implementation on `Theme` in `src/theme/mod.rs` (so existing themes keep compiling), forward it in `src/theme/restyle.rs`, call it via `render.rs`, and add it to `tests/themes.rs`. New hooks take `(f, ctx: &Context, view: &XView)`, read tokens from `ctx`, and keep their default body as a public function in `src/theme/defaults.rs`.
- New token: add it to the right group in `src/theme/tokens.rs` (settings groups use `settings!`) and to the presets `Tokens::plain()` / `Tokens::colorful()` if they need a value.
- A whole new theme: a struct implementing `Theme`, overriding only what differs, exported from `src/theme/mod.rs`.

## Releases
Releases are not documented in the repo. `CHANGELOG-OLD.md` says newer release notes live in GitHub Releases, and `Cargo.toml` sets `no_individual_tags` for cargo-workspaces. Suggested process: bump `version` in `Cargo.toml`, run the full CI command set above, publish via GitHub Release with notes grouped as Enhancements / Bug fixes / Breaking changes.

## Goals
paloma is a UI framework for command-line programs that run one action and exit: prompts, flows, messages, output blocks, spinners and progress bars, all customizable through themes and options. Only the prompts exist so far. It stays line-oriented: no full-screen TUI, no long-running or asynchronous apps. The architecture and phases are in [docs/plans/PAL-004-ui-framework.md](docs/plans/PAL-004-ui-framework.md); work is tracked in [ROADMAP.md](ROADMAP.md).
