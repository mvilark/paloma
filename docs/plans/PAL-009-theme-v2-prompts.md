# PAL-009: Theme v2 for prompts

Roadmap item: [PAL-009 in ROADMAP.md](../../ROADMAP.md). Umbrella plan: [PAL-004, Phase 4](PAL-004-ui-framework.md#pal-009--phase-4-theme-v2-for-prompts). Builds on PAL-006 (tokens, `Context`, `defaults`, `restyle`), PAL-007 and PAL-008.

## Executor

The planner (Opus 5.5, high effort) implements this plan itself, at the client's request (2026-10-11: "go ahead and implement it yourself").

| Criterion | Assessment |
| --- | --- |
| Design left open | None after this plan, but the token set is the public styling API of every prompt for years, so it is design-heavy. |
| Risk of subtle bugs | High: both built-in themes must stay byte-identical, which hinges on how `console` writes empty styles and nested styled text. |
| Size | Large: every prompt hook, the renderer, all prompts, both built-in themes, the Ultraviolet theme, goldens, docs. |
| Verification | Golden diffs line by line, plus rendering the Ultraviolet transcripts through a terminal emulator to prove they look the same. |

## Context and decisions

The prompt hooks are the last ones in the old shape: positional arguments, styles hard-coded in `ColorfulTheme`'s public fields, the paging counter written by the renderer outside the theme (convention break 9, Gap 7), and `fuzzy_matcher::skim::SkimMatcherV2` in the public API. Client decisions that apply:

- 2026-10-10: one breaking release, 0.13, for PAL-009 and PAL-010 together.
- 2026-10-10: paloma "is not opinionated. It is fully customizable, the developer never needs to write custom UI structs or tweak lower levels, and everything has sensible defaults with minimal code."
- 2026-10-10: the Ultraviolet example must not change its final output.
- Umbrella plan, Phase 4: the default layout becomes token-driven; `SimpleTheme` and `ColorfulTheme` become token presets on it, byte-identical to today; `ColorfulTheme`'s public style fields are replaced by tokens with a migration note; the paging counter goes through the theme; the fuzzy matcher leaves the public API; `UltravioletTheme` sets tokens and overrides only the hooks where its layout differs.

Decided here:

1. **Fewer hooks, one per layout, each `(f, ctx, view)`.** Answered lines of all prompts share one hook and one layout, and so do the item lines of all list prompts; the view says which prompt it is (`PromptKind`).
2. **No feature gates on hooks.** Views are plain data, so a theme compiles whatever features are on.
3. **Input errors are messages.** `format_error` goes; the renderer draws the error row with `format_message` and an error `MessageView`, styled by the message tokens. (PAL-010 turns this row into the message row.)
4. **The mask is a glyph.** `password_mask_glyph` goes; the `mask` glyph replaces it.
5. **Prompt tokens** follow the message, block and live tokens: a `base` `PromptStyle` and one group per prompt, falling back to the glyphs and palette roles.
6. **Two normalizations of the built-in looks**, because a regular layout cannot reproduce them; both are recorded in the snapshot diff and the PR:
   - `SimpleTheme`'s confirm lines lose a trailing space (`"Go? [y/n] "` becomes `"Go? [y/n]"`, `"Go? "` becomes `"Go?"`). Confirm hides the cursor, so nothing visible changes.
   - `ColorfulTheme`'s button-style confirm with an empty prompt now shows `›` on the question line like every other prompt with an empty prompt (it was an empty line).
7. **Ultraviolet bytes may change where the look does not.** Its answered lists are now styled item by item (as `ColorfulTheme` always did) and its matched fuzzy characters are nested styles. Proof of "unchanged final output": every changed snapshot line renders to the same characters and attributes in a terminal emulator (`pyte`).

## Files

- `src/theme/prompt.rs` (new): `PromptKind`, the views (`PromptView`, `InputPromptView`, `ConfirmPromptView`, `ConfirmButtonsView`, `FuzzySelectPromptView`, `ItemView`, `AnsweredView`, `Answer`).
- `src/theme/tokens.rs`: prompt glyphs, `PromptStyle`, `PromptTokens`, `PromptLook`, `Tokens::prompts`, presets.
- `src/theme/defaults.rs`: the prompt default layouts.
- `src/theme/mod.rs`: new hooks, old prompt hooks removed, re-exports.
- `src/theme/restyle.rs`: forward the new hooks.
- `src/theme/colorful.rs`: `ColorfulTheme` becomes a unit struct with colorful tokens; `simple.rs` unchanged.
- `src/theme/render.rs`: build a `Context` (tokens cached per renderer, stream from the term) and views; paging counter, error row and mask glyph through the theme.
- `src/prompts/*.rs`: pass views data; `fuzzy_select.rs` computes match indices itself.
- `tests/themes.rs`: same sheet lines on the new hooks; snapshots byte-identical except decision 6.
- `examples/ultraviolet/theme.rs`, `flows.rs`: tokens plus five overrides; answered lines through `format_answered`.
- `Cargo.toml`: drop `fuzzy_matcher::skim::SkimMatcherV2` from the allowed external types.
- `docs/USAGE.md` (Themes, migration), `DEVELOPMENT.md` (adding theme hooks and tokens), `ROADMAP.md`, `PAL-004` status note.

## Public API

```rust
// paloma::theme
pub enum PromptKind { Confirm, Input, Password, Select, MultiSelect, Sort, FuzzySelect }   // non_exhaustive

pub struct PromptView<'a>            { pub kind, pub prompt: &'a str, pub paging: Option<(usize, usize)> }
pub struct InputPromptView<'a>       { pub kind, pub prompt: &'a str, pub default: Option<&'a str> }
pub struct ConfirmPromptView<'a>     { pub prompt: &'a str, pub default: Option<bool>, pub buttons: bool }
pub struct ConfirmButtonsView        { pub yes_focused: bool }
pub struct FuzzySelectPromptView<'a> { pub prompt: &'a str, pub search: &'a str, pub cursor: usize }
pub struct ItemView<'a>              { pub kind, pub text: &'a str, pub active: bool, pub checked: Option<bool>, pub matches: Option<&'a [usize]> }
pub struct AnsweredView<'a>          { pub kind, pub prompt: &'a str, pub answer: Answer<'a> }
pub enum Answer<'a> { Text(&'a str), Bool(bool), List(&'a [&'a str]), Hidden(usize), Skipped }   // non_exhaustive
```

All views are `#[non_exhaustive]` with a `new(..)` taking every field in order.

Hooks on `Theme` (defaults in `theme::defaults` under the same names):

| New hook | Draws | Replaces |
| --- | --- | --- |
| `format_prompt(f, ctx, &PromptView)` | Question line of Select, MultiSelect, Sort, with the paging counter | `format_prompt`, `format_select_prompt`, `format_multi_select_prompt`, `format_sort_prompt`, the renderer's ` [Page x/y] ` |
| `format_input_prompt(f, ctx, &InputPromptView)` | Input and Password line | `format_input_prompt`, `format_password_prompt` |
| `format_confirm_prompt(f, ctx, &ConfirmPromptView)` | Confirm line; with `buttons` only the question | `format_confirm_prompt`, `format_confirm_prompt_no_hint` |
| `format_confirm_buttons(f, ctx, &ConfirmButtonsView)` | The button row | `format_confirm_buttons`, `format_confirm_button` |
| `format_fuzzy_select_prompt(f, ctx, &FuzzySelectPromptView)` | Search line with its cursor | `format_fuzzy_select_prompt` |
| `format_item(f, ctx, &ItemView)` | One item of any list prompt | `format_select_prompt_item`, `format_multi_select_prompt_item`, `format_sort_prompt_item`, `format_fuzzy_select_prompt_item` |
| `format_answered(f, ctx, &AnsweredView)` | The line an answered prompt leaves | every `format_*_prompt_selection*` |
| (`format_message`) | The input error row | `format_error` |
| (`mask` glyph) | Password mask | `password_mask_glyph` |

`Checked` is `Some(_)` for MultiSelect (checked) and Sort (picked), `None` otherwise. `matches` holds char indices of the search hits (FuzzySelect with highlighting on). `cursor` is a byte offset into `search`.

New glyphs (`Glyphs` setters and `GlyphName` variants): `prompt` (before an open prompt), `answered` (before an answered prompt), `prompt_suffix` (after the question), `answered_separator` (between question and answer), `focus` (before the active item), `checked`, `unchecked`, `cursor` (search cursor; empty means the character under the cursor is drawn in `cursor_style`), `mask`.

| Preset | prompt | answered | prompt_suffix | answered_separator | focus | checked | unchecked | cursor | mask |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `unicode()` | `?` | `✔` | `›` | `·` | `❯` | `✔` | `⬚` | (empty) | `*` |
| `ascii()` | `?` | `+` | `>` | `-` | `>` | `[x]` | `[ ]` | `\|` | `*` |
| `ascii_frames()` | (empty) | (empty) | `:` | `:` | `>` | `[x]` | `[ ]` | `\|` | `*` |

`PromptStyle` (`settings!`; groups `base`, `confirm`, `input`, `password`, `select`, `multi_select`, `sort`, `fuzzy_select` on `PromptTokens`, set with `Tokens::prompts(|p| p.base(|b| ..).sort(|s| ..))`; resolved with `PromptTokens::resolve(kind, &tokens) -> PromptLook`, all fields public):

- Glyph overrides, falling back to the glyph of the same name: `mark` (`prompt`), `answered_mark` (`answered`), `suffix` (`prompt_suffix`), `separator` (`answered_separator`), `focus`, `checked`, `unchecked`, `cursor`, `mask`.
- Words: `choices` (`(y/n)`), `choices_yes` and `choices_no` (the choices when the default is yes or no; default `choices`), `yes` / `no` (answers, default `yes` / `no`), `yes_button` / `no_button` (`Yes` / `No`), `button_brackets` (around the focused button, `("[", "]")`; the other button is padded with spaces to the same width), `default_brackets` (around an input default, `("(", ")")`), `hidden_value` (what an answered password shows instead of mask glyphs; default none), `empty_value` (what an answer that is skipped or an empty list shows; default none).
- Layout: `gap` (columns between the question and its suffix or separator, default 1), `item_indent` (columns before each item, default 0), `show_default` (a confirm shows the answer `Enter` gives after the suffix, default true).
- Styles, with their fallbacks: `mark_style` (accent), `label_style` (text), `suffix_style` (subtle), `hint_style` (subtle; input defaults and confirm choices), `default_style` (accent), `answered_mark_style` (success), `answered_label_style` (`label_style`), `separator_style` (subtle), `value_style` (text), `empty_style` (subtle), `focus_style` (accent), `active_style` (accent), `item_style` (text), `checked_style` (accent), `unchecked_style` (subtle), `match_style` (bold), `cursor_style` (accent), `text_style` (text; typed search), `paging_style` (none), `button_style` (text), `focused_button_style` (text).

Presets: `Tokens::plain()` sets `gap(0)`, `choices("[y/n]")`, `choices_yes("[Y/n]")`, `choices_no("[y/N]")`, `default_brackets(("[", "]"))`, `show_default(false)`; group `confirm`: `suffix("")`, `separator("")`; `password`: `hidden_value("[hidden]")`; `fuzzy_select`: `suffix("")`. `Tokens::colorful()` sets `mark_style` yellow, `label_style` bold, `value_style` green, `focus_style` green, `checked_style` green, `unchecked_style` magenta; groups `multi_select`: `focus("")`; `sort`: `focus("")`, `checked("❯")`, `unchecked(" ")`, `checked_style` green, `unchecked_style` none; `fuzzy_select`: `cursor_style` black on white.

`ColorfulTheme` becomes `#[derive(Clone, Copy, Debug, Default)] pub struct ColorfulTheme;` like `SimpleTheme`; it is written `ColorfulTheme` (`ColorfulTheme::default()` still compiles, though clippy suggests the shorter form). Migration: `ColorfulTheme { values_style: s, .. }` becomes `ColorfulTheme.restyle(|t| t.prompts(|p| p.base(|b| b.value_style(s))))`.

## Implementation steps

1. Tokens: glyphs and presets; `PromptStyle`, `PromptTokens`, `PromptLook`, `resolve` (member, then base, then glyphs and roles), following `LiveTokens` in `src/theme/tokens.rs`.
2. Views in `src/theme/prompt.rs`; re-export from `theme`.
3. Defaults in `src/theme/defaults.rs`. Shared rules: a mark is drawn only with a non-empty prompt; parts are joined by one space, skipping empty ones; a suffix or separator is preceded by `gap` spaces only when something precedes it. Specifics:
   - `format_prompt`: question, suffix, then ` [Page x/y] ` (the bracketed part in `paging_style`).
   - `format_input_prompt`: question, the default in `default_brackets` and `hint_style`, suffix, then one space when anything was written.
   - `format_confirm_prompt`: question, the choices for the default in `hint_style` (not with `buttons`), suffix, then ` yes` / ` no` in `default_style` when `show_default` and a default is set (not with `buttons`).
   - `format_confirm_buttons`: the yes button, two spaces, the no button.
   - `format_fuzzy_select_prompt`: question, suffix, a space when anything was written, the search in `text_style` with the cursor glyph in `cursor_style` inserted at `cursor`, or, with an empty cursor glyph, the character under the cursor (a space at the end) in `cursor_style`.
   - `format_item`: `item_indent` spaces; the focus glyph (or as many spaces) and a space, unless the focus glyph is empty; for `checked: Some(c)` the checked or unchecked glyph and a space (for Sort, `checked` counts only on the active item); the text in `active_style` or `item_style`, and with `matches` each character painted in that style around the character painted in `match_style` when it matched.
   - `format_answered`: answered mark and label, separator, then a space and the answer when anything was written before it: text, `yes` / `no`, the list joined by `, ` with each item in `value_style`, `hidden_value` or the mask glyph repeated, or `empty_value` for `Skipped` and an empty list.
4. `Theme`: replace the prompt hooks as in the table; `Restyled` forwards the new set.
5. `ColorfulTheme` unit struct with colorful tokens.
6. Renderer: cache `theme.tokens()` at creation; `Context` for the term's stream (`console::TermTarget::Stdout` → `Stream::Stdout`, otherwise stderr); one method per hook; the error row through `format_message`; the mask from the password look.
7. Prompts: call the renderer with view data; `FuzzySelect` keeps its matcher private and passes `fuzzy_indices` results.
8. Golden sheet: keep every line label, call the new hooks; removed hooks (`confirm_button`, `password_prompt_selection`, `password_mask_glyph`) become their replacements' lines.
9. Ultraviolet: `tokens()` from its palette and glyphs; overrides only `format_prompt`, `format_input_prompt`, `format_confirm_prompt`, `format_confirm_buttons` and `format_fuzzy_select_prompt` (the step counter, the input on its own line, the confirm layout); `answered(..)` calls `format_answered`.
10. Docs and roadmap.

## Tests and acceptance criteria

- `tests/snapshots/theme-*.txt`: byte-identical except the lines of decision 6 and the renamed lines of step 8.
- `tests/snapshots/ultraviolet-*.txt`: every changed line renders identically in `pyte` (same characters, colours, bold and italic).
- Unit tests: prompt look fallbacks (member, base, glyph, role); `format_item` with an indent and a wide focus glyph; `format_answered` with an empty list and `empty_value`; a restyled `ColorfulTheme` draws prompts exactly like the original; the Ultraviolet theme tests on the new hooks.
- Pty: `examples/select.rs` with paging, `examples/fuzzy_select.rs`, `examples/confirm.rs`, `examples/password.rs` with a validation error, and an Ultraviolet `setup` run, compared screen by screen with `main`.
- The full check set from `DEVELOPMENT.md`.

## Out of scope

Message row, placeholder, `done`, cancel and back results, presets and unattended mode, rich items, cursor style (PAL-010); step counters in views (PAL-015).

## Status notes

- 2026-10-11: Plan written; implementation by the planner on branch `claude/dazzling-galileo-dqa12c` (restarted from `main` after #11 merged).
- 2026-10-11: Implemented. Differences from the plan:
  - Added `list_separator_style` (the `, ` between list answers; default unstyled). Without it the Ultraviolet comma lost its text colour, which the golden test caught.
  - Ultraviolet's paging counter is now `fg-subtle`, as its design guide asks (Gap 7). No Ultraviolet command shows a paged list today.
  - Ultraviolet's sort showed `●` on every row while one was being moved, because the old hook received "a move is in progress" as `picked` for every item. The default layout shows it only on the moved row, which is what the design guide's "chosen, live" means.
  - `TermThemeRenderer::input_submitted` is gated on `password`, the only prompt using it; it warned with default features off.
  
  Checks: snapshots as planned; Ultraviolet snapshot changes render identically in `pyte`; pty runs of `select`, `paging`, `multi_select`, `sort`, `fuzzy_select`, `confirm`, `password`, `input`, `wizard` and Ultraviolet `setup` (Unicode and ASCII), `connect` and `favorites` match `main` screen by screen (characters, colours and attributes), apart from the sort fix and end-of-run cursor timing that differs between two runs of the same binary.
