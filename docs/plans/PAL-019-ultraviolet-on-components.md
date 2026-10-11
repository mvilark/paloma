# PAL-019: Ultraviolet example on paloma's components

Roadmap item: [PAL-019 in ROADMAP.md](../../ROADMAP.md). Part of the Ultraviolet update the umbrella plan schedules after its phases ([PAL-004](PAL-004-ui-framework.md), "Example:" bullets of Phases 1–5), brought forward for everything that exists now. Builds on PAL-006 to PAL-018; read their Status notes, which say how each phase was actually built.

## Executor

**Sonnet 5.5, high effort.**

| Criterion | Assessment |
| --- | --- |
| Design left open | Little: the mapping is below. The executor may need one or two small paloma tokens where a look cannot be reproduced; the rule for that is below. |
| Risk of subtle bugs | Medium: the example's output must look exactly the same, in five terminal setups. |
| Size | Medium and mechanical: replace hand-drawn output with components, set tokens. |
| Verification | Heavy but scripted: golden transcripts rendered in `pyte`, and pty runs before and after. |

The planner reviews the diff and reruns every check before pushing.

## Context and decisions

- 2026-10-11, client: move the Ultraviolet example onto paloma's components "right after PAL-018", instead of after every phase. The client asked whether `examples/ultraviolet/ui.rs` had shrunk; it has not (377 lines). The parts that need flows (PAL-015) or progress bars (PAL-011) wait for those phases.
- 2026-10-10, client: the example "must not change its final output, and must end up needing nothing lower-level than paloma's interface".
- PAL-004 principle 10: the example is an acceptance test. If a look cannot be built with paloma's tools, paloma is missing something.
- PAL-009 precedent: a change in escape-code bytes is acceptable when the transcript renders identically (same characters, colours, bold, italic, underline) in a terminal emulator.

Decided here:

1. **What moves:**
   - `ui::log`, `ui::hint` and `ui::cancelled` → `Message`.
   - `ui::section` → `paloma::section`; `ui::kv` → `KeyValue`; `ui::table` → `Table`; `ui::outro` → `Panel`.
   - `ui::link` → `paloma::text::link`; the wordmark's gradient → `paloma::text::gradient`.
   - `ui::Task` and `ui::task` → `paloma::task`.
   - The `--yes` and flag handling with the `answered(..)` helper → `.preset(..)`.
2. **What stays in `ui.rs`:** the Serrallonga icon and wordmark layout (app content), `progress` (until PAL-011), and a fake-work helper that sleeps between steps (app behaviour, not UI). `blank()` goes; blank lines come from block `space_before` tokens where a block follows, and from a plain `eprintln!()` elsewhere.
3. **What stays in `flows.rs`:** the setup step loop and `theme::set_step` (until PAL-015), the kill-switch and disconnect confirms skipped with `args.yes ||` (a preset would print an answered line that the transcripts do not have), and the unknown-server error.
4. **Missing tools go into paloma**, as tokens whose defaults keep the built-in themes byte-identical, recorded in this plan's Status notes and in `docs/USAGE.md`. One is known: the hint line is `Hint` and two spaces before the command, but `format_message` puts one space after its word. Add `word_gap: usize` to `MessageStyle` (default 1, resolved into `MessageLook`).
5. **Motion follows the example's own rule**, set once in `main`: `Options::live(|l| l.motion(motion))`.

## Files

- `examples/ultraviolet/ui.rs`: shrinks to the icon and wordmark, `progress`, and the fake-work helper.
- `examples/ultraviolet/flows.rs`: components and presets.
- `examples/ultraviolet/theme.rs`: message, block and live tokens in `tokens()`. The local `Palette` and `Glyphs` stay for `ui.rs` and the theme's own overrides.
- `examples/ultraviolet/main.rs`: live motion option; the Ctrl+C handler uses `paloma::terminal::restore()`.
- `src/theme/tokens.rs`, `src/theme/defaults.rs`: `word_gap` (and any other token found missing).
- `tests/snapshots/ultraviolet-*.txt`: may change bytes only (see acceptance).
- `docs/USAGE.md` (new tokens), `DEVELOPMENT.md` (the example's layout line), `ROADMAP.md`, `docs/plans/PAL-002-ultraviolet-example.md` (close the gaps this resolves in its Status notes).

## Implementation steps

1. **Record the baseline** before any change:
   - Build the example (`cargo build --example ultraviolet --all-features`) and copy the binary to the scratchpad.
   - Save the five golden transcripts (they are in `tests/snapshots/`).
2. **`word_gap`**: add it to `MessageStyle` and `MessageLook`; `defaults::format_message` writes `word_gap` spaces after the word. Built-in snapshots must stay byte-identical.
3. **Theme tokens** (`UltravioletTheme::tokens`):
   - **Messages:**
     - `hint`: word `Hint`, `word_gap(2)`, word style `subtle` italic, text style `accent_strong`, no glyph. The hint prints nested under the message it belongs to, so it gets the layout indent of 2.
     - The status glyphs and colours are already in the palette and glyphs.
   - **Blocks:**
     - `section`: title `text` bold, `space_before(1)`.
     - `key_value`: key `muted`, value `text`, gap 3.
     - `table`: header `text` bold, cells `text`, gap 3.
     - `panel`: title `accent_strong` bold, borders `faint`, trailing `subtle`, widths 40–64, `space_before(1)`.

     Check each against the old `ui.rs` code it replaces; the old code is the specification of the look.
   - **Live:** spinner `accent`, label `muted`, steps `muted` with the nested glyph `faint`, elapsed `subtle`, `step_indent(4)`.
4. **`flows.rs`:**
   - `ui::log(kind, msg)` → `paloma::success / warning / error / info(msg).print()`.
   - `ui::hint(cmd)` → `.nest(Message::hint(cmd))` on that message.
   - `ui::section(t)` → `paloma::section(t).print()`.
   - `ui::kv(..)` → `KeyValue::new().rows(..).print()` (stdout, as before).
   - `ui::table(..)` → `Table::new(header).rows(..).print()` (stdout).
   - `ui::outro(title, rows)` → `Panel::new(title)` with one `PanelRow` per row: `.status(Status::…)`, and `.trailing(format!("{:.1}s", d.as_secs_f32()))` where a duration exists.
   - `ui::task(running, steps, done, kind, secs)` → `paloma::task(running).start()?`, then the fake-work helper sleeps between `step(..)` calls, then `finish(status, done)?`, which returns the `Duration` used in the panel.
   - Presets:
     - email `.preset(args.email.clone().or_else(|| args.yes.then(|| "ada@example.com".into())))`;
     - password `.preset(args.yes.then(String::new))` (it draws the mask with `result_len` 8);
     - protocol, features, startup and favourites from the flag, or the current value under `--yes`;
     - server `.preset(..)` on the `FuzzySelect` with the index of `--server` or 0 under `--yes`.

     The `answered` helper and its imports go.
5. **`main.rs`:**
   - `paloma::set_options(.. .live(|l| l.motion(motion)))`.
   - The Ctrl+C handler calls `paloma::terminal::restore()`, then prints its cancelled line through `Message` (or keep the `ui::cancelled` look if `Message` cannot draw it; record why).
   - The unknown-command path uses `Message` with a nested hint.
6. **`ui.rs`**: delete everything that moved. `wordmark_with_icon` uses `paloma::text::gradient("Ultraviolet", from, to).style(bold).fallback(accent_strong bold)` with the dark or light endpoints it has today. `progress` stays as it is.
7. **Docs:**
   - `docs/USAGE.md`: `word_gap` and any other new token.
   - `DEVELOPMENT.md`: `ui.rs` now holds only the icon, wordmark and progress bar.
   - PAL-002 Status notes: which gaps are now closed (12 for everything except progress).
   - ROADMAP.

## Tests and acceptance criteria

- **Built-in snapshots** (`tests/snapshots/theme-*.txt`) byte-identical.
- **Ultraviolet snapshots:** after `UPDATE_SNAPSHOTS=1`, every changed file must render identically to the baseline in `pyte`. Use the PAL-009 render-comparison approach: replace `␛` with ESC and `\n` with `\r\n`, feed a 300-column screen, and compare every cell's character, foreground, background, bold, italics and underscore. Any visible difference is a bug to fix in tokens or paloma (decision 4), not a snapshot to accept.
- **Pty comparison against the baseline binary**, with the helper at `/tmp/claude-0/-home-user-paloma/cdf05a7f-88e2-52b1-9414-d6b777cecf3c/scratchpad/ptycmp.py`, `COLORTERM=truecolor`:
  - `ULTRAVIOLET_MOTION=0`: `setup` (the key list of PAL-009, Unicode and `--ascii`), `connect` (`es`, down, up, Enter, Enter), `favorites`, `disconnect` (Enter), `status`, `servers`, `about`. Screen by screen identical, except the end-of-run cursor timing noise noted in PAL-009.
  - With motion on (no `ULTRAVIOLET_MOTION`, no `CI`): the final screens of `setup` and `connect` identical to the baseline's; spinners visibly animate meanwhile.
  - Ctrl+C (`\x03`) while `connect`'s first task spins: the spinner line is erased, the cancelled line shows, the cursor is visible, exit code 130.
- **`wc -l examples/ultraviolet/ui.rs`:** report before and after.
- **The full check set** from `DEVELOPMENT.md`.

## Out of scope

- `progress` (PAL-011).
- The setup step loop, `set_step` and Esc-back handling (PAL-015).
- New Ultraviolet features (descriptions on its prompts, key legends): a later example pass if the client wants them.

## Status notes

- 2026-10-11: Plan written. To run right after PAL-018; afterwards work stops until the client resumes it.
