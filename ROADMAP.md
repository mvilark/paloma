# Roadmap

Living tracker for paloma work. Agents and contributors move an item to the next section as its state changes (Sketched ideas -> Planned -> In progress -> Done) and keep one entry per item, with a short note and a link to the PR when there is one.

## Sketched ideas

Rough ideas that are not yet scoped.

- **Global default theme.** Today every prompt defaults to `SimpleTheme` unless `with_theme()` is called on it (see `examples/input.rs`), so app developers repeat `with_theme()` on every prompt. Idea: let an app set one default theme once (for example a `set_default_theme()` or a `Theme`-holding global/builder) that prompts use when `with_theme()` is not called. Open questions: global state vs. a context object, thread safety, and how it interacts with explicit `with_theme()`.

## Planned

Scoped and agreed, not started.

_(none yet)_

## In progress

_(none yet)_

## Done

- Development guide ([DEVELOPMENT.md](DEVELOPMENT.md)), usage docs ([docs/USAGE.md](docs/USAGE.md)) and crate rename to `paloma` (PR #1).
