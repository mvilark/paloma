# PAL-016: Ultraviolet example uses what paloma already offers

Roadmap item: [PAL-016 in ROADMAP.md](../../ROADMAP.md). Example: [examples/ultraviolet/](../../examples/ultraviolet/).

## Goal

The example should do with paloma everything paloma can do today, and draw by hand only what paloma has no tool for yet. An audit (2026-10-11) found it uses the prompts, `set_theme`, `set_options` and the `Theme` hooks well, with four exceptions:

1. Lines printed for flags and `--yes` (`◇ Label · value`) were rebuilt by hand in `ui::answered`, although the theme already defines them in its answered-prompt hooks. If the theme changed, presets and real answers would drift apart.
2. Going back with Esc in `setup` re-asked Features and Connect on startup with fixed defaults instead of the user's earlier answers.
3. `--ascii` still printed `•` for info rows; the design guide requires an ASCII fallback for every glyph.
4. ASCII mode was detected with `cfg().g.cursor == "_"`, `ui::Task` kept an unused field alive, and the `--yes` feature list duplicated `FEATURES` as a literal.

## Proposed behavior

- Presets and `--yes` print through the hook of the prompt they stand for (`format_input_prompt_selection`, `format_password_prompt_selection_masked`, `format_select_prompt_selection`, `format_multi_select_prompt_selection`, `format_confirm_prompt_selection`, `format_sort_prompt_selection`) via `theme::app_theme()`.
- `setup` keeps its answers in its state and passes them back to the prompts (`defaults` for Features, `default` for Connect on startup).
- `Glyphs` gets `ascii` and `info` (`•` / `*`).

Not in scope, because paloma has no tool for it yet: messages, tasks, progress, sections, tables, the outro box (PAL-006 … PAL-011), the step counter and the step loop (PAL-015 flows), cancel detection (PAL-010).

## Implementation steps

1. `examples/ultraviolet/theme.rs`: `Glyphs::{ascii, info}`, `app_theme()`.
2. `examples/ultraviolet/flows.rs`: `answered(..)` helper over the theme hooks; state-backed defaults; feature list from `FEATURES`.
3. `examples/ultraviolet/ui.rs`: remove `answered`, use `g.info` and `g.ascii`, drop `Task::label`.
4. Tests: golden snapshots must be unchanged except the ASCII info glyph.

## Open questions / risks

None.

## Status notes

- 2026-10-11: Implemented on branch `claude/dazzling-galileo-dqa12c` (PR #9). Only `tests/snapshots/ultraviolet-ascii.txt` changed, by one glyph (`•` → `*`). Checked in a pty: Esc from Connect on startup returns to Features with the earlier picks checked.
