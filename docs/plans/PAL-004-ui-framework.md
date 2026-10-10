# PAL-004: From prompt library to CLI UI framework

Roadmap item: [PAL-004 in ROADMAP.md](../../ROADMAP.md). This is the umbrella plan. It sets the target architecture and splits the work into phases, each with its own roadmap ID (PAL-005 … PAL-011). Before a phase starts, its exact API gets a short plan of its own, as PAL-001 and PAL-002 did.

## Goal

Today paloma only asks questions. A real CLI also tells the user things (errors, warnings, info, success), shows work while it happens (spinners, progress) and prints results (key/value rows, tables, a summary box). Apps draw all of that by hand today: the Ultraviolet example's `ui.rs` is almost 400 lines of escape codes, threads and width maths that every app would have to rewrite.

After this plan, paloma is a UI framework for prompt-and-flow command-line programs:

- **Ready-made.** Every component works with no setup: `paloma::error("Port 3000 is in use")?`.
- **Customizable at every level.** Look and behaviour can each be changed app-wide, per component family, per component and per call, and a theme can take over the rendering of anything.
- **Families share a base.** Components that only differ in a few things (error / warning / info / success; spinner / progress bar; select / multi-select / sort / fuzzy select) share one base, so common settings are set once and overridden where needed.
- **The Ultraviolet example uses nothing below paloma's interface** (no escape codes, threads, cursor or width maths, no out-of-band globals), and its output does not change.

## Where paloma is today

### Purpose

paloma is a fork of dialoguer. It builds the interactive surface of *prompt-and-flow* CLIs: questions asked one at a time, a scrollback that reads as a record of what happened, results at the end. It is not a full-screen TUI and will not become one: it is for programs that run once for one specific action and exit when it is done. No long-running or asynchronous apps, no panes, no alternate screen.

### Principles already in the code and docs

1. **Works out of the box.** `Confirm::new().with_prompt("Continue?").interact()` needs no setup (`SimpleTheme`, built-in defaults).
2. **Look and behaviour are separate.** A `Theme` decides how things look; `Options` decide how they work (PR #8: "Themes decide how prompts look; options decide how they work").
3. **Settings cascade, most specific wins.** Behaviour: the prompt's builder method, then its group in `Options`, then `general`, then the built-in default. Look: `with_theme(&t)`, then `set_theme(t)`, then `SimpleTheme`. App-wide state is read when a prompt is shown, not when it is built.
4. **Override only what differs.** Every `Theme` method has a default; every option is an `Option` that falls through.
5. **One builder style.** `new()`, `with_theme`, `with_prompt`, setters taking `mut self -> Self`, and the `interact` / `interact_opt` / `interact_on` / `interact_on_opt` family returning `paloma::Result`.
6. **Terminal hygiene.** UI on stderr, results on stdout; the renderer owns line counting and clearing.
7. **A conservative, stable crate.** MSRV 1.85, few dependencies, optional ones behind features, only `console` types in the public API, docs build with warnings denied.
8. **Process.** Roadmap IDs, a plan file before code, a PR body in Before / After / How, `docs/USAGE.md` and `DEVELOPMENT.md` updated in the same PR, one example per feature, unit tests in the file they test.
9. **The Ultraviolet example is the acceptance test.** Its gap list ([PAL-002](PAL-002-ultraviolet-example.md)) is what drives the roadmap.

### How it works

`Prompt builder → TermThemeRenderer (line counting, clearing) → Theme::format_*(&mut dyn fmt::Write, positional args) → console::Term (stderr)`, with `PromptBase` + `report_methods!` shared by all prompts and `Options` holding app-wide behaviour.

### What does not scale

- **`Theme` is a flat list of positional-argument methods.** Each new piece of information a theme needs is a breaking signature change, so it gets worked around with a second method. That already happened: `format_password_prompt_selection_masked` exists only to add one argument. Every PAL-002 gap (hints, step counters, error state, disabled items) needs more arguments.
- **There are no design tokens.** A theme is either a set of `format_*` overrides or `ColorfulTheme`'s fixed fields. Ultraviolet had to build its own palette and glyph set, and its non-prompt output cannot share them with paloma.
- **A theme cannot know the context.** The step counter reaches the theme through a global `AtomicU32` in the example (`theme::set_step`), because no hook receives it.
- **The fuzzy-select hook leaks `fuzzy_matcher::SkimMatcherV2`** into the public API.

## Convention breaks to fix

Found while studying the repo. All of them are fixed in Phase 0 (PAL-005) unless marked otherwise.

**Process and docs**

1. `ROADMAP.md`: PAL-002 is still *In progress* although PR #7 merged, and its entry links a branch instead of the PR. *(Fixed in this PR.)*
2. PR #8 (app options, `PromptBase`, confirm buttons, password mask, input on its own line, errors under the input) has no roadmap ID or plan, and its PR body is not in Before / After / How. *(Recorded as PAL-003 in Done in this PR.)*
3. PAL-002's gap list is stale: PR #8 closed part of Gap 1 (input on its own line) and part of Gap 4 (errors under the input, `interact_text` only). Add a dated status note.
4. `README.md` repeats the `## License and Links` heading, does not credit dialoguer (PR #1 says it does), and still calls paloma "prompts and similar things".
5. `DEVELOPMENT.md` is out of date: it says the prompt's `theme` field is `Option<&dyn Theme>` (it is now `base: PromptBase`), the "Adding a prompt option / prompt type" steps do not mention `PromptBase`, `report_methods!` or option groups, and it says tests live only in `src/prompts/` (there are also tests in `options.rs`, `theme/app.rs` and `theme/render.rs`).
6. **CI does not run.** PRs #7 and #8 have no check runs at all, so none of the checks in `ci.yml` are enforced. The `semver` job would also fail once it runs: paloma is not on crates.io (the name is free), so there is no baseline to compare against. *(Needs a repository setting on your side: enable GitHub Actions. Phase 0 disables the semver job until the first release.)*
7. The Ultraviolet example's 4 unit tests never run: `cargo test --all-features` does not run example tests. Set `test = true` on the `[[example]]`.

**Code**

8. `USAGE.md` says "Input errors are always drawn on the line under the input", but `Input::interact` (not `interact_text`) and `Password` still print errors above the prompt with `render.error`. Make them match the doc.
9. The renderer writes ` [Page x/y] ` itself, bypassing the theme (DEVELOPMENT.md: "if it changes rendering, route it through the theme"). Add a theme hook (Gap 7). *(Phase 4, so the new hook gets the Theme v2 shape instead of being added twice.)*
10. `clear_preserve_prompt` assumes a 2-cell item prefix and measures items with `str::len` (bytes), so wrapped Unicode items and themes with a wider prefix leave ghost lines (Gap 8). Measure the formatted line instead.
11. Public items without docs, against the "public items need doc comments" rule: `Completion::get`, `BasicHistory`, `InputValidator::Err`, `PasswordValidator::Err`, `ColorfulTheme::fuzzy_match_highlight_style` (written with `//`). Add `#![deny(missing_docs)]` so the rule is enforced.
12. Doc typos: "Inctive item", "Formats out an error", and `ColorfulTheme::format_fuzzy_select_prompt` documented as "after selection" (it draws the live prompt).
13. `ColorfulTheme` ignores `Password::result_len` (it hard-codes 8 stars). It is the only built-in that ignores an option.
14. `Password` reads `options::current()` three times inside its loops. Every other prompt reads it once when shown.
15. Seven prompts repeat the same `new()` trick (`with_theme(&SimpleTheme)` then `base.theme = None`). Give `PromptBase` a constructor for both cases.
16. `lib.rs` holds a private `use paging::Paging` that prompts reach as `crate::Paging`; `lib.rs` should only hold public re-exports and module declarations.
17. In the example, `Kind::Info` prints `•` even in ASCII mode (the design guide requires an ASCII fallback for every glyph; visible with `--ascii`), ASCII mode is detected with `cfg().g.cursor == "_"`, and `Task` keeps an unused field alive with `let _ = &self.label`. These disappear when `ui.rs` is replaced (Phases 1-3).

## Target architecture

### Three kinds of components

| Kind | What it does | Components |
| --- | --- | --- |
| **Prompts** | ask, then collapse to a one-line record | `Input`, `Password`, `Confirm`, `Select`, `MultiSelect`, `Sort`, `FuzzySelect` (existing) |
| **Output** | print once and become scrollback | `Message` (success / info / warning / error, with an optional hint), `Section`, `KeyValue`, `Table`, `Panel` (summary box), inline helpers (`link`, `gradient`) |
| **Live** | redraw while work runs, then collapse to a static line | `Task` (spinner, nested steps, elapsed time), `Progress` (later) |

All three render through the theme, read the options, and write to a stream (stderr by default; data blocks like `KeyValue` and `Table` default to stdout, because they are results).

### Families and their bases

Components that only differ in a few things share a base. The base holds the shared state and builder methods (the pattern `PromptBase` + `report_methods!` already uses), and each family has one settings group in `Options` and one in the theme tokens.

| Family | Base | Members | Shared settings (examples) |
| --- | --- | --- | --- |
| prompts | `PromptBase` | all prompts | theme, report, hint, step counter, preset value, cancel behaviour |
| ↳ lists | `ListBase` | `Select`, `MultiSelect`, `Sort`, `FuzzySelect` | items (with description / disabled), page size, clear, paging |
| ↳ text | `TextBase` | `Input`, `Password` | validation display, cursor |
| messages | `Message` + `Status` | success, info, warning, error | stream, hint label, indent; per status: glyph, word, colour |
| blocks | `BlockBase` | `Section`, `KeyValue`, `Table`, `Panel` | stream, space before, indent, max width |
| live | `LiveBase` | `Task`, `Progress` | stream, motion, frame rate, elapsed format, cursor hiding, finish line |

`Status` (success / info / warning / error) is one enum shared by messages, task results and panel rows, so "a warning" looks the same wherever it appears.

### Behaviour: one cascade

`Options` keeps its closure-builder style and grows one group per family, with the existing per-prompt groups underneath:

```text
instance builder  →  component group  →  family group  →  general  →  built-in default
.stream(Stdout)      .error(|e| …)       .messages(|m| …)  .general(|g| …)
```

```rust
paloma::set_options(
    Options::new()
        .general(|g| g.motion(false).unattended(args.yes))   // app-wide
        .prompts(|p| p.report(true))                          // every prompt
        .confirm(|c| c.style(ConfirmStyle::Buttons))          // one prompt
        .messages(|m| m.stream(Stream::Stderr))               // every message
        .error(|e| e.stream(Stream::Stdout)),                 // one status
);
```

`general().report(..)` stays as a deprecated alias of `prompts().report(..)`. `general` becomes truly app-wide: motion, unattended mode, Unicode or ASCII, colour depth.

### Look: tokens, component tokens, hooks

Three layers, each optional, from cheapest to most powerful:

1. **Semantic tokens.** `Palette` (colour roles: `text`, `muted`, `subtle`, `faint`, `accent`, `accent_strong`, `success`, `info`, `warning`, `error`, as `console::Style`), `Glyphs` (named symbols with `Glyphs::unicode()` and `Glyphs::ascii()` sets, spinner frames included) and `Layout` (indent columns, line measure, gaps). Changing these restyles every component at once.
2. **Component tokens.** Per family, with a base and per-member overrides, falling back to the semantic tokens: `messages.base.indent`, `messages.error.glyph`, `blocks.panel.max_width`, `prompts.answered_separator`. This is the style side of "set it on the base, override it individually".
3. **Theme hooks.** `Theme::format_*` methods take over the rendering of one part entirely. Their default bodies use layers 1 and 2.

Tokens use the same closure-builder style as `Options`, so there is one way to configure things:

```rust
paloma::set_theme(
    ColorfulTheme::default().restyle(|t| t
        .glyphs(|g| g.error("✕").warning("▲"))
        .messages(|m| m.base(|b| b.label(None)).error(|e| e.hint_label("Fix")))),
);
```

`restyle` works on any theme, built-in or custom, without implementing the trait.

### Theme hook conventions (Theme v2)

New hooks, and the prompt hooks once they are migrated (Phase 4), follow one shape:

```rust
fn format_message(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &MessageView) -> fmt::Result {
    paloma::theme::defaults::format_message(f, ctx, view)
}
```

- `ctx: &Context` carries the resolved tokens, the target stream, the terminal width and the detected capabilities, plus `ctx.paint(&style, text)`, which styles for the right stream (`console` checks colour support per stream, which is why the example has `e()` and `o()` today). Hooks read tokens from `ctx`, never from `self`, so `restyle` can wrap a theme and the wrapped theme's hooks see the new tokens.
- `view: &XView` is a `#[non_exhaustive]` struct of everything the component knows (label, value, step, state, hint, error...). Adding a field later breaks nobody, which ends the `_masked`-style duplicate methods.
- The default bodies are public functions in `paloma::theme::defaults`, so an override can wrap the default instead of copying it.
- Views carry plain data only (match indices instead of `SkimMatcherV2`), which removes the external-type leak.

All new public structs (tokens, option groups, views) keep their fields private or `#[non_exhaustive]`, so adding a setting is never a breaking change. One crate-private `settings!` macro generates the `Option` fields, documented setters, getters and the fallback merge for both option groups and component tokens, so the two cascades cannot drift apart.

### Terminal layer

A crate-private `terminal` module owns everything below the components: capability detection (`tty`, colour depth from `COLORTERM` / `TERM`, `NO_COLOR`, `CI`, Unicode support from `TERM=linux` and legacy Windows consoles, `COLORFGBG`), the output streams, the cursor guard (restores the cursor on drop and on panic) and the live region. The detected `Capabilities` are public and read-only, so apps and themes can pick a palette from them; `Options::general` can override each one.

### Module layout

Each component is a vertical slice: one module owns its builder, view, option group, component tokens and default rendering. The registries (`Options`, `Tokens`, `Theme`, `lib.rs`) only list components. Adding a component means one new module plus one line in each registry.

```text
src/
  lib.rs              public re-exports and module declarations only
  error.rs
  settings.rs         (private) settings! macro and the cascade
  options.rs          Options, set_options, general; groups re-exported from components
  terminal/           (private) capabilities, streams, cursor guard, live region
  theme/              Theme trait (hooks grouped by family), Context, tokens/, defaults, restyle,
                      SimpleTheme, ColorfulTheme, render.rs
  prompts/            base.rs, list.rs, one module per prompt
  output/             message.rs, section.rs, key_value.rs, table.rs, panel.rs, text.rs
  live/               base.rs, task.rs, progress.rs
  validate.rs, history.rs, completion.rs, edit.rs, paging.rs
```

Public paths stay flat, as today: `paloma::Message`, `paloma::error`, `paloma::Task`, `paloma::theme::Tokens`.

## What the Ultraviolet example looks like at the end

Same output, no `ui.rs` drawing code, no global step counter, no raw terminal access. `theme.rs` keeps the Ultraviolet palette and glyph values (that is the design system) and the hooks where Ultraviolet's layout differs from paloma's default.

```rust
// main.rs
let caps = paloma::terminal::capabilities();
paloma::set_theme(UltravioletTheme::new(is_dark(&args), caps.depth(), args.ascii || !caps.unicode()));
paloma::set_options(
    Options::new()
        .general(|g| g.motion(motion).unattended(args.yes))
        .confirm(|c| c.style(ConfirmStyle::Buttons).on_shortcut(false))
        .password(|p| p.mask(true).result_len(8)),
);

// flows.rs
let email = Input::new()
    .with_prompt("Account email")
    .step(1, 5)
    .preset(args.email.clone().or(args.yes.then(|| "ada@example.com".into())))
    .validate_with(valid_email)
    .interact_text()?;

paloma::section("Setup")?;
let task = Task::new("Creating your profile...").start()?;
task.step("Generated device key")?;
let took = task.success("Created your profile")?;

Panel::new("Setup complete")
    .row(Status::Success, format!("Signed in as {email}"), Some(took))
    .row(Status::Info, "Connect on startup: On", None)
    .print()?;

paloma::error(format!("Server {id} not found")).hint("ultraviolet servers").print()?;
```

What stays in the example: argument parsing, theme choice (`--theme`, `ULTRAVIOLET_THEME`), fake data and sleeps, the wordmark text and the Serrallonga icon art, and the `ctrlc` handler (which calls paloma to clean up).

## Keeping the example's output unchanged

Phase 0 adds golden tests before anything moves:

- **Example transcripts.** An integration test (Unix only) runs every non-interactive command (`setup --yes`, `connect --yes`, `connect --server xx-nowhere-01`, `favorites --yes`, `disconnect --yes`, `status`, `status --json`, `servers`, `about`) under dark truecolor, light 256-colour, 16-colour, `NO_COLOR` and `--ascii`, with `CLICOLOR_FORCE=1` and `CI=1`, and compares stdout, stderr and the exit code byte for byte with files in `tests/snapshots/`. I checked that these runs are deterministic today (`CI=1` makes every duration `0.0s`). No new dependency: the test runs the example with `std::process::Command`.
- **Built-in theme output.** Unit tests that format every `Theme` hook with `SimpleTheme` and `ColorfulTheme` and compare against stored strings, so the Phase 4 rewrite of the defaults cannot change what existing users see.
- **Interactive flows.** A written checklist (setup flow with Esc back, fuzzy select, confirm buttons, password mask, a validation error, `^C` at a prompt and during a spinner) run in a real terminal before and after each phase, recorded in the PR as PAL-002 did.

Every phase must keep all snapshots identical. A difference is only allowed when it fixes a bug, and then it is listed in the PR and approved by you before merge.

## Phases

Each phase is one roadmap item, gets a short API plan before it starts, ships with its docs (`USAGE.md`, `DEVELOPMENT.md`) and an example, and moves the matching parts of the Ultraviolet example onto paloma.

| ID | Phase | Breaking | Size |
| --- | --- | --- | --- |
| PAL-005 | 0. Housekeeping and safety nets | no | S |
| PAL-006 | 1. Foundations and messages | no | L |
| PAL-007 | 2. Output blocks | no | M |
| PAL-008 | 3. Live tasks (spinners) | no | M |
| PAL-009 | 4. Theme v2 for prompts | **yes** | L |
| PAL-010 | 5. Prompt gaps from PAL-002 | **yes** (`Error`) | L |
| PAL-011 | 6. Progress bars | no | M |

Phases 1-3 are additive, so the new components can be used as soon as they land. The breaking changes are grouped in Phases 4 and 5 and released together as 0.13, so users migrate once.

### PAL-005 · Phase 0: Housekeeping and safety nets

- Fix convention breaks 3-8 and 10-16 above (9 moves to Phase 4).
- Add the golden tests described above, and run the example's tests (`test = true`).
- CI: disable the `semver` job until the first release; document in `DEVELOPMENT.md` that Actions must be enabled.
- No public API change.

### PAL-006 · Phase 1: Foundations and messages

The foundations land together with their first real user, so nothing is built on speculation.

- `settings!` macro; `Options` families (`general` app-wide, `prompts`, `messages`), with `general().report` deprecated.
- `terminal` layer: capabilities, streams, cursor guard.
- `Tokens` (`Palette`, `Glyphs`, `Layout`) and component tokens for messages; `Theme::tokens()`, `Context`, `theme::defaults`, `restyle`. `SimpleTheme` gets plain tokens (`error: …`, `warning: …`, like rustc and cargo); `ColorfulTheme` gets tokens matching its current colours and glyphs.
- `Status`, `Message`, shortcuts `paloma::success / info / warning / error`, `.hint(..)`, `print()` and `print_on(&Term)` (mirroring `interact_on`), all returning `paloma::Result`.
- `Theme::format_error` (used for validation errors) becomes "an error message" and renders through the same tokens.
- Crate description, keywords, README and crate docs reposition paloma as a CLI UI framework.
- Example: `ui::log` and `ui::hint` go; the palette and glyph set become paloma `Tokens`.

### PAL-007 · Phase 2: Output blocks

- `Section` (`# Title`), `KeyValue` (values aligned to the longest key plus a gap), `Table` (bold header, column gap), `Panel` (title, rows with an optional status glyph and a right-aligned trailing value, width clamped by layout tokens), `link` (OSC 8 when supported) and `gradient`.
- Spacing is a per-block `space_before` token, not global tracking of what was printed last.
- Example: `ui::section`, `ui::kv`, `ui::table`, `ui::outro`, `ui::link` and the gradient go.

### PAL-008 · Phase 3: Live tasks

- The live region: one live component at a time (the guide's "at most one spinner"), frames capped at 12 per second, the cursor hidden while it runs and restored on finish, drop and panic.
- `Task`: spinner, running label, elapsed time, nested `step` lines, `finish(Status, text)` plus `success` / `warning` / `fail` shortcuts returning the elapsed time.
- Motion off (option, `CI`, non-TTY): no redraws, each task printed once when it finishes.
- `paloma::terminal::restore()` for an app's Ctrl+C handler. paloma installs no signal handler itself.
- A prompt shown while a task runs is refused with an error, not drawn over the spinner.
- Example: `ui::Task`, `ui::task` and `ui::cancelled` go.

### PAL-009 · Phase 4: Theme v2 for prompts

- Prompt hooks move to the `(f, ctx, view)` shape. The default layout becomes token-driven, and `SimpleTheme` and `ColorfulTheme` are expressed as token presets on that layout, byte-identical to today (checked by the Phase 0 golden tests).
- `ColorfulTheme`'s public style fields are replaced by its tokens, with a migration note (`ColorfulTheme { values_style, .. }` becomes `.restyle(|t| …)`).
- The paging counter and the item prefix width go through the theme (Gaps 7 and 8). The fuzzy matcher leaves the public API.
- Example: `UltravioletTheme` sets tokens and overrides only the hooks where its layout differs.

### PAL-010 · Phase 5: Prompt gaps from PAL-002

- Hints and key legends (Gap 2); rich items with a description and a disabled state (Gap 3); validation while typing, with the error in place of the hint and the input kept (Gap 4); step counters in the view (Gap 6, removes the example's `AtomicU32`); cursor style (Gap 9).
- Cancel and back (Gap 5): `Error::Cancelled` and `#[non_exhaustive]` on `Error`, Esc for `Input` and `Password`, the cancelled record drawn by the theme, the live prompt cleaned up on `^C`.
- Presets and unattended mode (Gap 10): `.preset(Option<T>)` prints the collapsed record and skips the prompt; with `general().unattended(true)` prompts return their default and print the record. Without either, a prompt on a non-terminal still returns an error, as today.
- Example: `ui::answered` goes, and so does the rest of `ui.rs` except the progress bar.

### PAL-011 · Phase 6: Progress bars

- `Progress` on `LiveBase`: a total, `set` / `inc`, a label, a percentage, easing toward the real value (never backwards) and the head glint as live tokens. Motion off prints the finished line once.
- Example: `ui::progress` goes, and with it `ui.rs`.

### Later, not scheduled (Sketched ideas)

- OSC 11 background query to choose dark or light (Gap 11).
- Several live lines at once (multi-progress), if a real app needs it.
- Changing the terminal background colour only while the program works, and restoring it on exit. Undecided; PAL-002 left backgrounds out of scope.

## Implementation steps

For every phase:

1. Write its short API plan (copy of `TEMPLATE.md`) and get it agreed.
2. Implement on its own branch, one PR per phase (the plan PR may come first, as for PAL-001 and PAL-002).
3. Tests: unit tests in each component module for its rendering (components render to a `String` through the theme, so no terminal is needed), plus the golden tests from Phase 0.
4. Docs: a section per component in `USAGE.md`; in `DEVELOPMENT.md`, the layout above and an "Adding a component" checklist (module, `Options` group, tokens, `Theme` hook, `lib.rs` re-export, example, `USAGE.md`).
5. Move the matching parts of the Ultraviolet example onto paloma and confirm the snapshots are unchanged.

## Open questions / risks

Direction decisions (agreed 2026-10-10, see status notes):

1. **Breaking release.** paloma is not published and the example is the only known custom theme, so a one-time redesign of `Theme` (Phase 4) and `Error` (Phase 5) costs little now and much later. **Agreed:** allow both, released together as 0.13.
2. **Live rendering engine.** Recommended: paloma's own small engine (one live region, `Task` and `Progress`, no new dependency). The alternative is building on `indicatif`: mature and supports many bars at once, but it styles through template strings that bypass the theme and tokens, its types cannot appear in paloma's public API, and its terminal handling would compete with the prompt renderer. **Agreed:** paloma's own engine.
3. **Scope.** Recommended: stay line-oriented (prompts, output, live lines) and never become a full-screen TUI (no layouts, panes or alternate screen). That keeps scrollback as the record and keeps the crate small. **Agreed:** paloma is for programs that run one action and exit; terminal GUIs and long-running or asynchronous apps are out of scope.

Risks:

- **Phase 4 is the largest change.** The golden tests from Phase 0 are what make it safe; it does not start until they exist.
- **Global state.** Theme, options and the live region are app-wide. Tests that touch them stay in one test function per binary, as `theme/app.rs` already does.
- **Terminal detection differs across platforms.** Capabilities can always be overridden with `Options::general`, and the golden tests pin the environment.

## Status notes

- 2026-10-10: Plan written after studying the repo. Convention breaks 1 and 2 fixed in the same PR (roadmap only).
- 2026-10-10: spideyincrocs agreed to all three direction decisions: one breaking release (0.13) for `Theme` v2 and `Error`; paloma's own live engine, no `indicatif`; line-oriented scope only, for programs that run one action and exit (no terminal GUI, no long-running or async apps). A background colour while the program works may be considered later (PAL-014).
- 2026-10-10: Phase 0 (PAL-005) needs no API plan of its own (no public API change beyond docs and one additive `ColorfulTheme` override); its scope is the list above.
- 2026-10-10: Phase 0 implemented on branch `claude/dazzling-galileo-dqa12c`. Golden tests first (built-in theme hooks; the example in 5 terminal setups × 11 commands), then the fixes. Two output changes, both intended: `ColorfulTheme` now shows `result_len` stars instead of always 8 (default unchanged); list prompts no longer erase lines above the list when an item is long in bytes but not in columns (a bug the pty check found). Convention break 10 turned out to be that bug. Snapshots of the example are unchanged.
