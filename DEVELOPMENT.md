# Development

No CONTRIBUTING.md exists yet. This file covers contributor workflow; for API usage see [docs/USAGE.md](docs/USAGE.md). Note: the repo is the "interacteur" fork, but `Cargo.toml` still names the crate `dialoguer` (v0.12.0) and examples import `dialoguer::`; a rename has not happened yet.

## Layout
- `src/lib.rs` - public re-exports and feature gates (`#![deny(clippy::all)]`).
- `src/prompts/` - one file per prompt: `confirm`, `input`, `select`, `multi_select`, `sort`, `password`, `fuzzy_select`.
- `src/theme/` - `Theme` trait (`mod.rs`), `SimpleTheme`, `ColorfulTheme`, and `render.rs` (internal `TermThemeRenderer` that prompts draw through).
- `src/validate.rs`, `history.rs`, `completion.rs`, `edit.rs` (`Editor`), `paging.rs`, `error.rs`.
- `examples/` - one runnable example per feature (`cargo run --example wizard`).
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
Tests are unit tests inside `src/prompts/*.rs` (`#[cfg(test)]`); there is no `tests/` directory. Coverage alias: `cargo test-cover` (needs cargo-llvm-cov).

CI (`.github/workflows/ci.yml`) also runs: tests on Linux/macOS/Windows, MSRV check (Rust 1.85, `cargo check --lib --all-features`), cargo-semver-checks, cargo-check-external-types (only `console` and `fuzzy_matcher::skim::SkimMatcherV2` may appear in the public API), and cargo-deny audit.

## Conventions
- MSRV 1.85; do not use newer language/std features.
- Builder style: `new()`, `with_theme(&dyn Theme)`, `with_prompt(..)`, option setters taking `mut self -> Self`, and `interact` / `interact_opt` / `interact_on` / `interact_on_opt` terminals returning `crate::Result`. Prompts derive/impl `Clone` and `Default` where sensible.
- Public items need doc comments; docs build with warnings denied.
- Gate optional code with `#[cfg(feature = "...")]` and update both `lib.rs` and docs.rs metadata (`all-features = true`).
- Avoid leaking new third-party types in the public API (external-types check).

## Adding a prompt option
1. Add a field to the prompt struct and its `with_theme` constructor in `src/prompts/<prompt>.rs`.
2. Add a documented builder method following the existing pattern.
3. Use it in `interact_on*`; if it changes rendering, route it through the theme (below).
4. Add a unit test in the file's `tests` module, update an example if useful, and mention it in `docs/USAGE.md`.

## Adding a new prompt type
Create `src/prompts/<name>.rs`, register it in `src/prompts/mod.rs`, re-export from `src/lib.rs` (feature-gated if it adds a dependency, plus a `[[example]]` entry with `required-features`), add `format_<name>_*` methods with defaults to `Theme`, and add an example.

## Adding or changing theme options
- New styling field: add it to `ColorfulTheme` (`src/theme/colorful.rs`) and its `Default` impl; use it in the relevant `format_*` override. Adding a public field is a semver break for struct-literal users without `..Default::default()`, so check cargo-semver-checks.
- New rendering hook: add a `format_*` method with a default implementation on `Theme` in `src/theme/mod.rs` (so existing themes keep compiling), call it via `render.rs`.
- A whole new theme: a struct implementing `Theme`, overriding only what differs, exported from `src/theme/mod.rs`.

## Releases
Releases are not documented in the repo. `CHANGELOG-OLD.md` says newer release notes live in GitHub Releases, and `Cargo.toml` sets `no_individual_tags` for cargo-workspaces. Suggested process: bump `version` in `Cargo.toml`, run the full CI command set above, publish via GitHub Release with notes grouped as Enhancements / Bug fixes / Breaking changes.

## Goals
More functionality (new prompt types and options), deeper customization via `Theme`, and more built-in themes, while keeping the MSRV and the semver-checked public API stable.
