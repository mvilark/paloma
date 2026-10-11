# PAL-002: Ultraviolet example app

Roadmap item: [PAL-002 in ROADMAP.md](../../ROADMAP.md). Design system: [docs/design/ultraviolet.md](../design/ultraviolet.md).

## Goal

Build a realistic example program that shows how close paloma can get to a real design system today, and produce a precise list of what it cannot express yet. The example is a fake VPN CLI themed with **Ultraviolet**. The gap list at the end of this plan is the input for the next roadmap items: we extend the theming system until the example no longer needs any workaround.

Out of scope, as agreed: background colours (`selection-bg`, badge fills, `term-bg`) and anything that does not apply to a terminal (fonts, line height, preview sizes).

## Proposed behavior

### The program

`examples/ultraviolet/`, run with `cargo run --example ultraviolet --all-features -- <command>`. It fakes all network work with short sleeps and canned data.

| Command | What it shows | Prompts used |
| --- | --- | --- |
| *(no command, first run)* | wordmark and Serrallonga icon, then `setup` | |
| `setup` | a 5-step flow with `01/05` counters, then `# setup` tasks and an outro box | `Input` (account email, validated), `Password`, `Select` (protocol), `MultiSelect` (features), `Confirm` (connect on startup) |
| `connect [--server <id>]` | server search, a spinner with nested `↳` step lines, a progress bar for the key exchange, outro box with `✓` badge and key/value rows | `FuzzySelect` (server), `Confirm` (kill switch is off, connect anyway?) |
| `favorites` | reorder favourite servers | `Sort` |
| `disconnect` | confirm, one task line | `Confirm` |
| `status [--json]` | key/value rows on stdout, or JSON with `--json` | |
| `servers` | a table with a bold header row | |
| `about` | wordmark (gradient) and the Serrallonga icon | |

Failure paths are part of the demo: `connect --server xx-nowhere-01` prints `✕ Server xx-nowhere-01 not found` and a `hint` line with a command to paste; an outro with the kill switch off shows a `▲` line.

Global flags and environment, all from the guide: `--theme dark|light` / `ULTRAVIOLET_THEME`, `COLORFGBG`, `--ascii` (also `TERM=linux`), `--yes` (accept every default, no prompts), `--json`, one flag per prompt (`--email`, `--protocol`, `--server`, ...), `COLORTERM`, `NO_COLOR`, `TERM=dumb`, `CI` and `ULTRAVIOLET_MOTION=0`. Prompts and progress go to stderr, results to stdout. `^C` prints `◇ … · cancelled` and exits with code 130.

### The theme

`UltravioletTheme` is a struct in the example that implements `paloma::theme::Theme` directly (not a `ColorfulTheme` with changed fields, because `ColorfulTheme` hard-codes its line layout and the guide's layout differs). It is built from two parts chosen at startup:

- **Palette**: every token from the guide (`fg`, `fg-muted`, `fg-subtle`, `fg-faint`, `accent`, `accent-strong`, `accent-dim`, `success`, `warning`, `error`, `wordmark-from/to`) as a `console::Style`, for dark and light, at three depths: 24-bit (`COLORTERM=truecolor|24bit`), 256 and 16 colours, using the guide's fallback table. `console` already supports `Color::TrueColor`, `Color256`, italic, strikethrough and drops styling for `NO_COLOR`, dumb terminals and non-TTY output.
- **Glyphs**: the Unicode set or the ASCII set from the guide's glyph table.

The app calls `paloma::set_theme(UltravioletTheme::detect(&args))` once. Everything that is not a paloma prompt (sections, spinner, progress, task lines, outro box, tables, wordmark, icon) is drawn by a small `ui` module in the example with the same palette and glyphs.

## Mapping: what paloma can express today

Status key: **Theme** = expressible in `UltravioletTheme`. **Partial** = expressible with a visible compromise. **Example** = drawn by the example outside paloma, because paloma has no concept of it. **Gap** = cannot be done faithfully without changing paloma (numbered, see the gap list).

| Guide rule | Status | How, or why not |
| --- | --- | --- |
| One hue, colour only on results, colour on the glyph only | Theme | The theme only uses `accent` and the `fg` tiers in prompts; status colours only on the `✕` glyph of a validation error. |
| No status words (`error:` etc.) | Theme | `format_error` writes `✕ message`; the default `error: ` prefix is gone. |
| Active prompt `◆ Label`, bold label, no trailing colon | Theme | `format_*_prompt`. |
| Collapse answered prompts to `◇ Label · value` | Theme | `format_*_prompt_selection`. `Select` defaults to `report(false)`, so the example sets `report(true)`. |
| Yes/no shows `[Y]es / [N]o`, no key legend | Theme | `format_confirm_prompt`. |
| Focused option `▸` in `accent`, focused text `accent-strong`, others `fg-muted` | Theme | `format_select_prompt_item`, `format_fuzzy_select_prompt_item`. |
| Checkboxes `■` / `□` | Theme | `format_multi_select_prompt_item`, `format_sort_prompt_item`. |
| Options indented `col-2` | Partial | Item lines can start with two spaces, but the renderer's wrap maths assumes a 2-cell item prefix (Gap 8). |
| Input indented `col-2` on the line under the label | Gap 1 | Input prompts must fit on one line. |
| Hint line under a prompt (italic, `fg-subtle`) | Gap 2 | No hint concept in paloma. |
| Option descriptions in lists | Gap 3 | Items are plain strings. |
| Key legend line | Gap 2 | Same as hints. |
| Validation error replaces the hint, `›` turns `error`, input kept | Gap 4 | Errors print as a separate line above the re-rendered prompt and the input is cleared; the theme cannot tell the prompt is in an error state. |
| Validate as the user types | Gap 4 | Validation only runs on Enter. |
| `◇ Label · cancelled` / `skipped` | Gap 5 | No cancelled or skipped rendering; `^C` returns an error and leaves the live prompt on screen. |
| `esc` goes back one prompt | Partial | `Select`, `MultiSelect`, `Sort`, `FuzzySelect` and `Confirm` return `None` on Esc through `interact_opt`, so the example's flow steps back. `Input` and `Password` have no Esc handling (Gap 5). |
| `01/05` step counters in `fg-subtle` | Gap 6 | The theme knows nothing about the flow; the example has to put the counter inside the label text, where it is styled as `title`. |
| Paging counter in `fg-subtle` | Gap 7 | The renderer writes ` [Page 1/2] ` itself, unstyled. |
| Disabled options with strikethrough | Gap 3 | No disabled items. |
| Blinking block cursor | Gap 9 | Paloma uses the terminal's own cursor. The example can request a blinking block with an escape code before prompts, but cannot reset the blink on each key. |
| `--yes` accepts defaults, unattended runs | Gap 10 | Prompts return an error when there is no terminal; the example must skip prompts itself. |
| Fuzzy search cursor | Theme | `format_fuzzy_select_prompt` draws it (static, no blink). |
| ASCII fallback | Theme | Glyph set chosen at startup. |
| Dark/light, colour depth, `NO_COLOR`, non-TTY | Theme | Palette chosen at startup; `console` strips styles when needed. |
| OSC 11 background query | Gap 11 | Needs a terminal query with a timeout; the example uses flag, env and `COLORFGBG` only. |
| Prompts on stderr | Theme | Paloma's default. |
| `# section` titles, blank-line rhythm | Example | `ui::section`. |
| Spinner, running-label shimmer, elapsed time, hidden cursor | Example | `ui::task`: a thread redrawing one line at most 12 times a second. |
| Task results `✓` / `▲` / `✕`, nested `↳` lines | Example | `ui::task` and `ui::log`. |
| Progress bar with glint and easing | Example | `ui::progress`. |
| Outro box with badge, right-aligned durations | Example | `ui::outro` (badge without fill, since backgrounds are out of scope). |
| Key/value rows, tables, links (OSC 8, underline) | Example | `ui::kv`, `ui::table`, `ui::link`. |
| Wordmark gradient and shimmer, Serrallonga icon | Example | `ui::wordmark`, `ui::icon`. The guide does not include the icon art, so the example draws an 8-line version. |
| Motion off for `CI`, non-TTY, `ULTRAVIOLET_MOTION=0` | Example | `ui` prints each task once when it finishes. |

## Gap list (input for the next roadmap items)

Ordered by how visible the compromise is in the example.

1. **Multi-line prompt layout.** Let a theme put the label and the input on separate lines. Today the input cursor maths assumes the formatted prompt is one line.
2. **Hints and key legends.** A per-prompt hint (`with_hint(..)`) and a theme hook to format it, plus a key-legend hook.
3. **Rich items.** Items with a description and a disabled state, and theme hooks for both.
4. **Validation state.** Live validation while typing, the error drawn in place of the hint, the input kept, and a way for the theme to style the prompt in its error state.
5. **Cancel and back.** Render `cancelled` / `skipped` through the theme, clean up the live prompt on `^C`, and give `Input` and `Password` an Esc result.
6. **Flow context.** Step counters (`01/05`) passed to the theme, so it can style them.
7. **Paging counter through the theme.** Move ` [Page x/y] ` into a `Theme` method.
8. **Item prefix width.** Stop assuming a 2-cell item prefix when clearing wrapped items.
9. **Cursor style.** Let a theme choose the input cursor (block, blink).
10. **Unattended mode.** A way to make prompts return their defaults (for `--yes`, CI and non-TTY runs).
11. **Terminal detection helpers.** Background (OSC 11) and colour-depth detection, so themes can pick a palette.
12. **Non-prompt output.** Sections, task lines with spinner, progress, outro box, tables, wordmark. These are the biggest block of "Example" rows above; whether paloma grows into them is the main decision for the follow-up roadmap.

## Implementation steps

1. **Files.**
   - `examples/ultraviolet/main.rs`: argument parsing (by hand), theme detection, command dispatch, `^C` handling, exit codes.
   - `examples/ultraviolet/theme.rs`: palette tokens per theme and depth, glyph sets, `UltravioletTheme` implementing every `Theme` method the example uses.
   - `examples/ultraviolet/ui.rs`: non-prompt output listed in the mapping table.
   - `examples/ultraviolet/flows.rs`: one function per command, with fake data.
   - `Cargo.toml`: an `[[example]]` entry `ultraviolet` with `required-features = ["fuzzy-select", "password"]`.
   - `Cargo.toml`: `ctrlc` as the only new dev-dependency. Its handler restores the cursor, prints `◇ … · cancelled` and exits with code 130, including while a spinner runs. Flags are parsed by hand; spinner and progress are hand-rolled with `console`.
2. **Tests.** Unit tests inside `theme.rs` for palette selection (depth and dark/light), the ASCII glyph set, and the collapsed-line format (`◇ Label · value`); run with `cargo test --example ultraviolet --all-features`. CI already compiles all examples (`cargo check --examples --all-features`).
3. **Manual checks**, recorded in the PR: dark and light, `COLORTERM` unset (256 colours), `TERM=xterm` with 16 colours, `NO_COLOR=1`, `--ascii`, piped output, `CI=1`, `--yes`, `^C` during a prompt and during a spinner.
4. **Docs.** A short "Ultraviolet example" paragraph in `docs/USAGE.md` (Themes section) pointing at the example, and the example listed in `DEVELOPMENT.md`'s layout section.

## Open questions / risks

- **Hand-rolled spinner and progress** add a fair amount of example code. That is deliberate: it shows exactly what Gap 12 would move into paloma.
- **Icon art** is not in the guide, so the example's Serrallonga icon is our own drawing.

## Status notes

- 2026-10-10: Plan written. Style guide stored as `docs/design/ultraviolet.md`.
- 2026-10-10: spideyincrocs chose `ctrlc` as the only new dev-dependency (no `clap`, no `indicatif`).
- 2026-10-10: Implemented on branch `claude/pal-002` (`examples/ultraviolet/`, `ctrlc` dev-dependency). Differences from the plan: `UltravioletTheme` also uses `●`/`○` for `Sort`; the Serrallonga icon is a small hand-drawn version; `accent-dim` is defined but unused. Manual checks done in a pty: fuzzy select, the setup flow (Esc/step back not exercised), `^C` at a prompt and during a spinner (exit 130), `--yes`, piped output, `CI=1`, `--ascii`, `NO_COLOR`. Not checked: 16-colour terminal look and light theme on a real light terminal.
- 2026-10-10: PR #8 (recorded as PAL-003) closed part of the gap list. Gap 1: a theme can now put the input on its own line (`format_input_prompt` may contain `\n`), which the example uses. Gap 4: errors are drawn under the input instead of above the prompt, and the input is still cleared; live validation and the error replacing a hint remain open. PAL-005 extended the error placement to `Input::interact` and `Password`, and fixed Gap 8 (clearing now counts each line's real width). The remaining gaps are scheduled in [PAL-004](PAL-004-ui-framework.md).
- 2026-10-11: Gap 12 (non-prompt output) is closed for everything except the progress bar. PAL-019 moved the example's sections, messages and hints, key/value rows, tables, outro panels, links, the wordmark gradient and task lines with spinner onto paloma's `section`, `Message`, `KeyValue`, `Table`, `Panel`, `text::link` / `gradient` and `task`, and `--yes` and flag answers onto `.preset(..)` (Gap 10). Still drawn by the example: the progress bar (until PAL-011), the Serrallonga icon and wordmark layout (app content), and the setup step loop (until PAL-015). The "Example" rows of the mapping table above describe the first version.
