# PAL-017: Prompt outcomes, items and legends

Roadmap item: [PAL-017 in ROADMAP.md](../../ROADMAP.md). Umbrella plan: [PAL-004, Phase 5](PAL-004-ui-framework.md#pal-010--phase-5-prompt-gaps-from-pal-002), second of three parts. Needs [PAL-010](PAL-010-message-row.md) merged first (frames in the renderer, the message row); read the Public API sections of [PAL-009](PAL-009-theme-v2-prompts.md) and PAL-010 before starting.

## Executor

**Sonnet 5.5, high effort.**

| Criterion | Assessment |
| --- | --- |
| Design left open | None: every name, default and edge case is below. |
| Risk of subtle bugs | Medium: the `^C` cleanup, and disabled items in paging and wrap-around navigation. |
| Size | Large but repetitive: the same changes in seven prompts. |
| Verification | Unit tests with a non-terminal `Term`, and pty checks. |

The work is mostly applying one pattern seven times, which suits Sonnet; high effort covers the edge cases. The planner reviews the diff and reruns every check before pushing.

## Context and decisions

What a flow (PAL-015) needs from each prompt, plus the remaining gaps of the Ultraviolet example (PAL-002 Gaps 2, 3, 5, 10). Client decisions (2026-10-10, umbrella plan):

- "**What an answered prompt leaves.** `.done(Done::Collapse)` (the default: one line with the answer), `Done::Keep` (the prompt stays as it was) or `Done::Erase` (nothing). It replaces today's `report` and `clear` flags, also in `Options`."
- "`Error::Cancelled` and `#[non_exhaustive]` on `Error`, an Esc ("back") result for every prompt including `Input` and `Password`, the cancelled state drawn by the theme, the live prompt cleaned up on `^C`."
- "`.preset(Option<T>)` skips the prompt and shows the value as an answered prompt would (as configured with `done`); with `general().unattended(true)` prompts return their default the same way. Without either, a prompt on a non-terminal still returns an error, as today."
- "Key legends (Gap 2), rich items with a description and a disabled state (Gap 3)."
- paloma installs no signal handler (PAL-008); "`^C` cancels the whole flow … `run()` returns `Error::Cancelled`, so the app can exit with code 130" (PAL-015).

Decided here:

1. **`Esc` erases the prompt** and `interact_opt` returns `Ok(None)`, whatever `done` says. A flow going back must not leave the old prompt on screen. Today `Confirm` leaves a "skipped" line, which PAL-005 recorded as a bug.
2. **`^C` is read as a key.** Prompts read with `Term::read_key_raw()`, which returns `Key::CtrlC` instead of raising SIGINT. They then erase themselves, draw the cancelled line if the theme has one, show the cursor and return `Err(Error::Cancelled)`, both from `interact` and from `interact_opt`.
3. **Unattended runs need a default.** A prompt with no preset and no default returns `Err(Error::NoDefault)`.
4. **Presets and unattended values are used as given**: they are not validated and `on_change` is not called.
5. **Rich items for Select, MultiSelect and FuzzySelect**, through a public `IntoItem` trait implemented for `Item` and every `Display` type, so `.items(&["a", "b"])` keeps working. `Sort` keeps plain items: disabling an item makes no sense when reordering.
6. **Legends are off by default.** They are words in tokens (so a theme can translate them), drawn by a hook on the frame's last line.

## Files

- `src/error.rs`: `Error` gains `Cancelled`, `NoDefault`, `#[non_exhaustive]`.
- `src/prompts/base.rs`: `done` and `legend` replace `report` in `PromptBase` and `base_methods!`. Shared helpers: `fn done(&self) -> Done` (resolution) and `fn finish(..)` (draws per `done`).
- `src/prompts/item.rs` (new): `Item`, `IntoItem`, the disabled-aware navigation helper.
- `src/prompts/*.rs`: `preset`, `interact_opt` for Input and Password, `^C`, `Esc`, disabled items, legends; `clear` and `report` go.
- `src/options.rs`: `done` and `legend` replace `report`; `GeneralOptions` holds `unattended` (its deprecated `report` goes).
- `src/lib.rs`: re-export `Done`, `Item`, `IntoItem`.
- `src/theme/prompt.rs`: `Answer::Cancelled`; `ItemView` gains `description`, `disabled`; `LegendView`.
- `src/theme/tokens.rs`: the new prompt tokens below.
- `src/theme/defaults.rs`, `mod.rs`, `restyle.rs`: `format_legend`; item descriptions and disabled items; the cancelled answer.
- `src/theme/render.rs`: `keep()` (leave the last frame without its row); the legend line in frames.
- `src/paging.rs`: reserve the legend row.
- `tests/themes.rs`, snapshots; `examples/*.rs` that used `report` or `clear`; `examples/items.rs` (new); `examples/ultraviolet/main.rs` and `theme.rs`.
- `docs/USAGE.md`, `DEVELOPMENT.md`, `ROADMAP.md`.

## Public API

```rust
// paloma::Error
#[non_exhaustive]
pub enum Error {
    IO(io::Error),
    /// The user pressed Ctrl+C. Exit with code 130 to follow the shell convention.
    Cancelled,
    /// Unattended mode asked a prompt that has no default and no preset.
    NoDefault,
}
// Display: "IO error: {io}", "cancelled", "no default for an unattended prompt".
// From<Error> for io::Error: Cancelled → ErrorKind::Interrupted, NoDefault → ErrorKind::InvalidInput.

// paloma::Done
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Done {
    /// One line with the answer (`format_answered`). The default.
    #[default] Collapse,
    /// The prompt stays as it was when answered, without its message row.
    Keep,
    /// Nothing stays.
    Erase,
}
```

On every prompt (`base_methods!`): `.done(Done)` and `.legend(bool)`, replacing `.report(bool)`. A prompt without a question (Select, MultiSelect and Sort accept none) treats `Collapse` as `Erase`, as today. `Options`: `PromptOptions`, `ConfirmOptions` and `PasswordOptions` replace `report: bool` with `done: Done` and `legend: bool`; `GeneralOptions` becomes `{ unattended: bool }` (default false). Resolution: the prompt's own value, its group, `prompts`, then the default (`Collapse`, legend off).

Removed: `Select::clear`, `MultiSelect::clear`, `FuzzySelect::clear` (use `done(Done::Keep)` or `done(Done::Erase)`), every `report`, and `GeneralOptions::report`.

Esc and back: `interact_opt` and `interact_on_opt` on every prompt, newly on `Input` and `Password`. `Esc` erases the prompt and returns `Ok(None)`. `q` keeps working where it does today (Select, MultiSelect, Sort, Confirm). `FuzzySelect` with vim mode keeps `Esc` for entering vim mode. `interact` ignores `Esc`.

Presets: `.preset(value: Option<T>)`. `Some` skips the prompt, needs no terminal, and draws per `done` (`Keep` draws like `Collapse`, since nothing was shown). `None` asks as usual.

| Prompt | `T` | Unattended default |
| --- | --- | --- |
| `Confirm` | `bool` | `.default(..)` |
| `Input<T>` | `T` | `.default(..)` |
| `Password` | `String` | none (always `NoDefault`) |
| `Select` | `usize` | `.default(..)` if set |
| `MultiSelect` | `Vec<usize>` | the items checked by default (may be empty) |
| `Sort` | `Vec<usize>` | the original order |
| `FuzzySelect` | `usize` | `.default(..)` if set |

Rich items:

```rust
// paloma::Item
#[derive(Clone, Debug)]
pub struct Item { /* text, description: Option<String>, disabled: bool */ }
impl Item {
    pub fn new(text: impl Into<String>) -> Self
    /// Extra text drawn after the item.
    pub fn description(mut self, text: impl Into<String>) -> Self
    /// A disabled item is shown but cannot be focused, chosen or checked.
    pub fn disabled(mut self, disabled: bool) -> Self
}
/// Something a list prompt can show as an item: an `Item`, or any `Display` value as its text.
pub trait IntoItem { fn into_item(self) -> Item; }
impl IntoItem for Item { .. }
impl<T: std::fmt::Display> IntoItem for T { .. }
```

`Select::item(impl IntoItem)` and `items(impl IntoIterator<Item = impl IntoItem>)`, likewise `MultiSelect::item` / `items` / `item_checked` / `items_checked` and `FuzzySelect::item` / `items`. `Item` must never implement `Display`; say so in a comment.

Theme:

- `Answer::Cancelled`: drawn by the default `format_answered` like `Skipped`, but with `cancelled_value` in `cancelled_style`. The renderer only draws it when `cancelled_value` is set.
- `ItemView` gains `description: Option<&'a str>` and `disabled: bool` (and `new` takes them after `matches`).
- New hook `format_legend(f, ctx, view: &LegendView) -> fmt::Result`, default `defaults::format_legend`. `LegendView { kind, keys: &'a [(&'a str, &'a str)] }` (key, action), `#[non_exhaustive]`, with `new`.

New `PromptStyle` tokens:

| Token | Type | Default | Purpose |
| --- | --- | --- | --- |
| `cancelled_value` | `&'static str` | none | After the question when `^C` cancels; none erases. |
| `cancelled_style` | `Style` | `subtle` | |
| `item_description_style` | `Style` | `subtle` | |
| `item_description_gap` | `usize` | 2 | Columns between an item and its description. |
| `disabled_style` | `Style` | `muted` plus strikethrough | Disabled item text. |
| `legend` | `&'static [(&'static str, &'static str)]` | per prompt, below | The keys and actions. |
| `legend_back` | `(&'static str, &'static str)` | `("esc", "back")` | Added when `Esc` goes back (`interact_opt`). |
| `legend_separator` | `&'static str` | `"  "` | Between entries. |
| `legend_key_style` | `Style` | `muted` | |
| `legend_style` | `Style` | `subtle` | Actions and separators. |

Default legends:
- Input, Password: `[("enter", "submit")]`.
- Select: `[("↑↓", "move"), ("enter", "choose")]`.
- MultiSelect: `[("↑↓", "move"), ("space", "check"), ("a", "all"), ("enter", "submit")]`.
- Sort: `[("↑↓", "move"), ("space", "pick up"), ("enter", "submit")]`.
- FuzzySelect: `[("↑↓", "move"), ("enter", "choose")]`.
- Confirm: `[]` (no legend line at all).

`Tokens::plain()` replaces `↑↓` with `up/down` in its groups.

## Implementation steps

1. **`Error`** with the two variants, `Display`, the `io::Error` conversion, `#[non_exhaustive]`. Fix every exhaustive `match` on it in the crate.

2. **`Done` and options.**
   - `Done` lives in `src/options.rs`, re-exported at the root.
   - Replace `report` as above; `Options::report(..)` becomes `Options::done(own, group) -> Done`.
   - `base_methods!(group)` loses its `$default` closure. The no-question rule is in each list prompt: `if self.prompt.is_none() { Done::Erase }`.
   - `fn legend_enabled(&self) -> bool` resolves the same way.
   - Unit tests in `options.rs` (replacing the `report` test) for the cascade.

3. **Renderer:**
   - `keep()`: redraw the live frame without its row lines (and legend), then move the cursor below it, write `\r\n`, and reset the frame state without erasing.
   - Frames take an optional legend line, drawn after the row through `format_legend` with the resolved look's legend (plus `legend_back` when `Esc` goes back).
   - `Paging::set_reserved` counts it.

4. **`finish`** (crate-private, on `TermThemeRenderer`): `finish(done: Done, kind, prompt, answer: Answer)`. `Collapse` erases and draws the answered line; `Keep` calls `keep()`; `Erase` erases. Every prompt's successful `Enter` path calls it instead of its own `clear` / `report` code.

5. **Keys:** in every prompt replace `term.read_key()` with `term.read_key_raw()`.
   - `Key::CtrlC`: `erase()`; draw `answered(.., Answer::Cancelled)` if the look has `cancelled_value`; show the cursor; flush; return `Err(Error::Cancelled)`.
   - `Key::Escape` in the `_opt` variants (and `q` where it works today): `erase()`, show the cursor, return `Ok(None)`.
   - Input and Password: add `interact_opt` / `interact_on_opt` returning `Result<Option<T>>` (`Result<Option<String>>`). `interact` / `interact_on` call a shared private function with `allow_back: bool`, like Select does today.

6. **Presets and unattended:**
   - At the top of each `_interact_on`, before the `is_term` check:
     - if a preset is set, draw it with `finish_preset(done, kind, prompt, answer)` and return it (`Erase` draws nothing; `Collapse` and `Keep` draw the answered line);
     - otherwise, if `crate::options::current().general.unattended == Some(true)`, use the default from the table, or return `Err(Error::NoDefault)`.
   - `finish_preset` writes with `term.write_line` and needs no terminal.
   - Read the options once per call, as `DEVELOPMENT.md` requires.

7. **Items** (`src/prompts/item.rs`):
   - `Item`, `IntoItem`.
   - `fn next_enabled(items: &[Item], from: usize, forward: bool) -> Option<usize>`: wraps around, `None` when every item is disabled. Unit-test it.
   - Select, MultiSelect and FuzzySelect store `Vec<Item>`. Arrow keys, `Tab`, `j` / `k` and page moves land on the next enabled item in that direction. The start position is the default, or the next enabled item from it.
   - Space on a disabled item does nothing; `a` toggles only enabled items. `Enter` with no enabled item does nothing.
   - FuzzySelect matches on `text` only, and skips disabled items when moving.
   - The answered line shows the text.
   - Disabled items keep their checked state: a disabled item checked by default stays checked and is returned.

8. **Theme:**
   - Default `format_item`:
     - text in `disabled_style` when `disabled` (with matches inside it as usual);
     - then, when `description` is set, `item_description_gap` spaces and the description in `item_description_style`.
   - `format_legend`: `message_indent` spaces, then `key action` pairs (key in `legend_key_style`, a space, action in `legend_style`) joined by `legend_separator` in `legend_style`.
   - `Answer::Cancelled` in `format_answered`.
   - Forward the new hook in `restyle.rs`.
   - Existing snapshot lines must stay byte-identical (no description, not disabled).

9. **Examples and docs:**
   - Fix every example using `report` or `clear`.
   - New `examples/items.rs`: a Select with descriptions and one disabled item, legends on.
   - Ultraviolet:
     - `theme.rs` sets `cancelled_value("cancelled")` with `cancelled_style` subtle italic, its legend styles (`legend_key_style` muted, `legend_style` subtle italic), `disabled_style` (muted plus strikethrough) and `item_description_style` (subtle italic).
     - `main.rs`: where a command's `Result` is handled, `Err(Error::Cancelled)` prints the existing `ui::cancelled()` and exits with 130, as its Ctrl+C handler does for spinners. Keep that handler.
     - Its `report` and `clear` calls, if any, become `done`. Do not move its `--yes` handling onto presets yet (that is the Ultraviolet update after all phases), but it must still compile and its golden transcripts must not change.
   - `docs/USAGE.md`: "What an answered prompt leaves", "Back and cancel", "Presets and unattended runs", "Items", "Key legends", and the migration from `report` / `clear`. `DEVELOPMENT.md`: `item.rs`, and that prompts read keys with `read_key_raw`. ROADMAP.

## Tests and acceptance criteria

- **Golden snapshots:** existing lines byte-identical; the Ultraviolet transcripts unchanged. New sheet lines:
  - `answered(Go?, Cancelled)` with and without a `cancelled_value` (restyle the theme in the test);
  - `item(b, description)`, `item(b, disabled)`, `item(b, disabled, active)`;
  - `legend(Select)` and `legend(MultiSelect, back)`.
- **Unit tests:**
  - `Error` conversion;
  - `Done` resolution; `next_enabled`;
  - presets and unattended defaults for every prompt, driven through `Term::read_write_pair` (Unix only, `#[cfg(unix)]`) with a `Vec<u8>` writer, asserting the returned value and the answered line written (with `console::set_colors_enabled(false)`);
  - `NoDefault` for Password and for a Select without a default.
- **Pty checks** (Python `pty` + `pyte`):
  - `select` example: `^C` (`\x03`) while open. The prompt is erased, the cursor is visible, and the process exits (the example unwraps the error, so a non-zero exit is expected; assert the screen, not the code).
  - Ultraviolet `setup` (`COLORTERM=truecolor ULTRAVIOLET_MOTION=0`): `^C` at the email prompt shows its cancelled line and exits 130. `Esc` on the confirm step erases it and goes back to Features, with no `Skipped` line left.
  - `examples/items.rs`: down skips the disabled item; the description and legend show.
  - `examples/input.rs` with `done(Done::Keep)` set temporarily in a scratch copy: the typed line stays after `Enter`.
  - Every example and Ultraviolet `connect` / `favorites` screen-by-screen equal to the PAL-010 branch for the same keys.
- **The full check set** from `DEVELOPMENT.md`.

## Out of scope

- Background checks and cursor style: [PAL-018](PAL-018-timed-input.md).
- Flows, step counters, `back_to`: PAL-015.
- Moving the Ultraviolet example's `--yes` and flag handling onto presets: the Ultraviolet update after all phases.

## Status notes

- 2026-10-11: Plan written.
