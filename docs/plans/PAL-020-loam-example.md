# PAL-020: Loam, a cozy garden journal example

Roadmap item: [PAL-020 in ROADMAP.md](../../ROADMAP.md). A second full example app beside Ultraviolet ([PAL-002](PAL-002-ultraviolet-example.md)), built on every paloma feature that exists after PAL-006 to PAL-019, PAL-015 and the cancel defaults of PR #17. Read every plan's Status notes first: they say how each feature was actually built.

## Executor

**Sonnet 5.5, medium effort** (the client's choice, 2026-10-11; first assessed as high).

| Criterion | Assessment |
| --- | --- |
| Design left open | Little: the app, its commands, its data and its theme are specified below. Exact copy and glyph substitutions (where a glyph is not one column wide) are left to the executor within the stated spirit. |
| Risk of subtle bugs | Low to medium: the app only calls paloma. The risk is a look that paloma's tokens cannot express; the rule for that is below. |
| Size | Large but mechanical: about eight commands, a theme made of tokens, mock data, golden transcripts. |
| Verification | Golden transcripts in five terminal setups; pty runs of every interactive command. |

The planner reviews the diff and reruns every check before pushing.

## Context and decisions

- 2026-10-11, client: "a new example app that uses all features of paloma that we have so far, and make sure that it will make sense to implement the task spinner and progress bar when we have them. The example app can be anything cool. It should use mock data that doesn't save anywhere. And it should have a new theme that is as different as possible to the Ultraviolet one, but something very aesthetically pleasing. Maybe something more organic and cozy. This app should not have a ui file at all, at least for now, forcing it to use only the native functions of paloma."
- Spinners already exist (PAL-008 `paloma::task`), so Loam uses them now. Progress bars (PAL-011) do not exist yet. Loam has three places where a progress bar is the natural component; today they use a task with steps, written so PAL-011 can swap them in (see "Progress seams").
- PAL-004 principle 10: an example app is an acceptance test. If Loam cannot get a look from paloma's tokens and hooks, paloma is missing something, and the fix goes into paloma, never into the app.

Decided here:

1. **The app is Loam**, a garden journal: plan a garden, sow seeds, water beds, harvest, keep notes, check the almanac. All data is canned in `data.rs` and lives in memory for one run. Nothing is read from or written to disk; the editor command writes to a temporary file through `paloma::Editor` only.
2. **No UI code in the app.** The example has no `ui.rs` and writes no escape codes, no styled text and no layout of its own.
   - Everything shown goes through paloma: prompts, flows, messages, blocks, `text::link`, `text::gradient`, tasks.
   - The only styling code is in `theme.rs`, and it only builds `Tokens`.
   - Plain `println!` is allowed for results on stdout (a value another program might read), never for decoration.
3. **The theme is tokens only.** `LoamTheme` implements `Theme` with nothing but `tokens()`: no `format_*` overrides. Where a look needs more than the tokens offer, add a token to paloma, with a default that keeps both built-in themes and Ultraviolet byte-identical, and record it in the Status notes. As a last resort a hook override is allowed only with the client's OK.
4. **It looks nothing like Ultraviolet.**

   | | Ultraviolet | Loam |
   | --- | --- | --- |
   | Colour | One electric violet on near-black | A warm earth palette in several hues: moss, clay, honey, rain and parchment |
   | Glyphs | Hard geometric shapes (`◆ ◇ ▸ ■`) | Floral and soft (`✿ ❀ ❧ ◌`) |
   | Spinner | Braille | Rounded arcs |
   | Boxes | Rounded boxes | Dotted rules with floral corners |
   | Copy | Terse, capitalised | Lowercase, warm sentences |
   | Spacing | Tight | Airy: one blank line before blocks |

## Files

- `examples/loam/main.rs`: argument parsing, the theme and options setup, the command dispatch, the exit code.
- `examples/loam/theme.rs`: `LoamTheme` (tokens only): palette for day and night at three colour depths, Unicode and ASCII glyphs.
- `examples/loam/commands.rs`: one function per command.
- `examples/loam/data.rs`: the canned garden: seeds, beds, weather, harvest log, journal entries.
- `Cargo.toml`: `[[example]] name = "loam"`, `path = "examples/loam/main.rs"`, `required-features = ["fuzzy-select", "password", "history", "completion", "editor"]`, `test = true`.
- `tests/loam.rs` and `tests/snapshots/loam-*.txt`: golden transcripts (Unix), like `tests/ultraviolet.rs`.
- `docs/design/loam.md`: the theme's design guide, shorter than `ultraviolet.md`: principles, colour roles with hex values, glyphs, spacing, copy.
- `docs/USAGE.md` (mention Loam beside Ultraviolet), `DEVELOPMENT.md` (layout line), `ROADMAP.md`.
- Only if needed: new paloma tokens (decision 3), with their docs and unit tests.

## The app

`loam [command] [flags]`. With no command, the first run: the wordmark, then `init`.

Flags:
- `--yes` / `-y`: unattended: `Options::general(|g| g.unattended(true))` plus presets. Also on when stderr is not a terminal.
- `--ascii`: ASCII glyphs; also on with `TERM=linux`.
- `--day` / `--night`: palette; default night, or from `LOAM_THEME`.
- `--plain`: draw with `SimpleTheme`, to show that the app has no look of its own.
- `--name <garden>`, `--zone <zone>`: presets for `init`.
- `--json` for `status`.

Exit codes: 0 done, 1 a command failed (a message says why), 2 unknown command, 130 cancelled (paloma's default; Loam sets `cancel_message` to a hint-style `put your trowel down · nothing was changed` line).

### Commands and the features they use

**`init`: plant a new garden.** A `Flow` with `.title("new garden")`, `.keep_last(3)` and `.done(Done::Collapse)` (the summary line `❀ new garden — 6 steps`).
1. `name`:
   - `Input` with `.placeholder("the back plot")`, `.description("what do you call this patch of earth?")` and a validator (2 to 24 letters, digits or spaces).
   - `.on_change` with `.change_delay(400)` and `.pending("asking the neighbours…")`: a fake 600 ms "is the name taken" check against `data::TAKEN_NAMES`.
   - Preset from `--name`.
2. `zone`: `Select` of climate zones as `Item`s with descriptions ("mild winters, long summers"), one disabled (`polar · coming soon`), `.legend(true)`. Preset from `--zone`.
3. `beds`: `Input<u8>` with `.default(4)` and a validator (1 to 12).
4. `sun`: `Confirm` with buttons: "does it get full sun?".
5. `shade` (`step_if` sun is no): `Select` "how many hours of sun?".
6. `soil test`: no prompt. A task "testing the soil…" with steps `pH 6.4`, `nitrogen low`, `drainage good`.
   - On the first run with more than 8 beds it fails: `flow::back_to("beds", Message::warning("the soil can feed 8 beds this year").nest(Message::hint("try fewer beds, or add compost first")))`.
   - This step asks nothing, so `Esc` from the next step skips it.
7. `companion`: `Password` "a garden password, for sharing" with `.mask(true)`, a confirmation, and the nested `on_change` rule list (at least 8 characters, one digit).

After the flow: a `section("your garden")`, a `KeyValue` of the answers, and `paloma::success("the back plot is ready").nest(Message::hint("next: loam plant"))`.

**`plant`: sow seeds.**
- `FuzzySelect` over the 40-seed catalogue (`data::SEEDS`), with `.placeholder("search seeds…")` and highlighted matches. Paging shows on a short terminal.
- `MultiSelect` of companion plants with `.defaults(..)`, items with descriptions, one disabled (out of season), `.validate` (at least one), `.legend(true)`.
- `Confirm` "sow them in bed 2?" in the classic `(y/n)` layout (not buttons), so both confirm layouts appear in the app.
- Then the **sowing seam** (below), and a `Panel` "sown" with a row per plant (success status, trailing "sprouts in 7–10 days").

**`water`: water the beds.**
- `Table` of beds (bed, plants, last watered, moisture) on stdout.
- `MultiSelect` "which beds?" with dry beds checked by default.
- Then the **watering seam**, and `paloma::info("the rain barrel is half full")`.

**`harvest`: plan the harvest order.**
- `Sort` "what to pick first" over ripe crops.
- A task "picking…" whose result line is `warning` when a crop is overripe.
- Then `KeyValue` yields.
- Inside the task, a `Confirm` "the zucchini is huge. pick it anyway?" shows that a prompt pauses a running spinner (PAL-008).

**`journal`: write a note.**
- `Input` with in-memory `BasicHistory` pre-seeded from `data::JOURNAL` (`↑` brings old notes) and `Completion` of plant names (`Tab`).
- `--long` opens `paloma::Editor` instead, and prints the result back.
- Ends with `done(Done::Keep)`, so the note stays as typed.

**`status`:**
- A `KeyValue` of the garden.
- A frost warning: `paloma::warning("frost tonight").nest(Message::hint("cover the seedlings in bed 3"))`.
- An error for a bed that needs attention, with a nested hint.
- `--json` prints canned JSON on stdout instead.

**`almanac`:**
- The wordmark: `text::gradient("loam", …)` from moss to honey, with a one-line tagline.
- A `Panel` "this week" with status rows and trailing values ("rain · thursday", "first frost · 12 days").
- `text::link` to the paloma repository.

**`seeds`:** a `Table` of the catalogue on stdout.

Coverage check: every prompt type, each confirm layout, presets and unattended mode, `done` in all three modes (Collapse default, Keep in `journal`, Erase for the transient "which beds?" via `step_done` or the prompt's own `done`), `Esc` back, `back_to`, `step_if`, `keep_last`, the flow summary, the counter, the message row (description, `on_change` with and without delay, `pending`, validation), placeholders, legends, rich items, paging, history, completion, the editor, masks and confirmation, cursor shape, every message status with nesting, every block, link, gradient, tasks with steps / every finish status / `clear` / a prompt during a task, `Options` (app-wide and per group), `terminal::capabilities` (to pick ASCII and colour depth), `restyle` (`--plain` and the day palette), and the cancel defaults (exit 130 with `cancel_message`). List any feature you could not use, with the reason, in the Status notes.

### Progress seams

Three places where a progress bar (PAL-011) is the right component. Each is written today as one small function in `commands.rs` that takes the number of units and runs a task with one step per unit. A doc comment starts with `// PAL-011:` and says which progress call replaces it. When progress bars land, only these functions change:

| Seam | Today | With PAL-011 |
| --- | --- | --- |
| `sow(seeds)` in `plant` | task "sowing…", one step per seed ("sowed basil"), result "sowed 6 seeds" | a progress bar "sowing" over the seed count, the label naming the current seed |
| `water(beds)` in `water` | task "watering…", one step per bed with litres | a progress bar per bed or over total litres |
| `fetch_catalogue()` on the first `plant` of a run | task "fetching the seed catalogue…" with steps per 10 seeds | a progress bar over 40 seeds, eased |

Durations are faked with `thread::sleep` scaled by a `motion` flag, as in Ultraviolet: no sleeping when `CI` is set or the stream is not a terminal, so transcripts are instant and every duration reads `0.0s`.

## Theme

`examples/loam/theme.rs` builds `Tokens` from scratch (`Tokens::default()` plus every group), choosing:
- the **palette** by `--day` / `--night` and colour depth: 24-bit, 256, 16 (`terminal::capabilities(Stream::Stderr).depth()`);
- the **glyphs** by `--ascii` / `capabilities().unicode()`.

Palette roles (24-bit hex; pick the nearest 256-colour index, and for 16 colours a named colour):

| Role | Night (on dark) | Day (on light) | Name |
| --- | --- | --- | --- |
| `text` | `#EDE3D1` | `#3B2F24` | parchment / espresso |
| `muted` | `#B8A992` | `#6B5B4A` | oat |
| `subtle` | `#8C7E6A` | `#8E7D69` | bark |
| `faint` | `#5A5045` | `#D8CBB6` | soil |
| `accent` | `#9DBF7A` | `#5E8040` | moss |
| `accent_strong` | `#C6E3A0` | `#3F6127` | sprout |
| `success` | `#A3C98F` | `#4E7D4E` | leaf |
| `info` | `#9EC3C9` | `#4F7F88` | rain |
| `warning` | `#E8B860` | `#A86F12` | honey |
| `error` | `#D9785B` | `#B5502F` | terracotta |

Glyphs (Unicode / ASCII). Every Unicode glyph must measure 1 column with `console::measure_text_width`; replace any that does not with a glyph of the same feel and record it:

| Purpose | Unicode | ASCII |
| --- | --- | --- |
| prompt mark | `✿` | `*` |
| answered mark | `❀` | `o` |
| prompt suffix | `~` | `~` |
| answered separator | `—` | `-` |
| focus | `❧` | `>` |
| checked / unchecked | `◉` / `◌` | `(x)` / `( )` |
| search cursor | `▏` | `_` |
| password mask | `⁎` | `*` |
| section | `❦` | `#` |
| nested step | `⤷` | `-` |
| info / success / warning / error | `◦` / `✓` / `☂` / `✗` | `i` / `+` / `!` / `x` |
| spinner | `◜ ◝ ◞ ◟` | `. o O o` |
| box: horizontal, vertical, corners | `┈`, `┊`, `·` (all four) | `-`, `:`, `.` |

Tokens beyond the palette and glyphs:
- **Prompts:**
  - mark in `accent`, question in `text` (not bold: Loam never shouts);
  - answered mark `subtle`, answered question `muted`, separator `faint`, value `text`;
  - items `muted`, active item `accent_strong`, focus `accent`;
  - hints, descriptions and placeholders in `subtle` italic;
  - `message_indent(2)`, `item_indent(2)`, `invalid_suffix_style` `error`;
  - cursor shape `Bar`, blinking;
  - legend keys `muted`, actions `subtle` italic;
  - item descriptions `subtle` italic, disabled items `faint` with strikethrough;
  - the step counter `subtle`, written `3 of 7` if a `step_format` token exists. Otherwise keep `03/07` and record that a format token would be needed.
  - Confirm buttons as words, `yes` / `no`, focused in `accent` bold.
- **Messages:** no words, glyphs only; hint word `tip` (`word_gap(2)`) in `subtle` italic.
- **Blocks:** `space_before(1)` for sections and panels; section titles `accent_strong`; table headers `muted` italic; panel title `accent`.
- **Live:** spinner `accent`, label `muted`, steps `subtle`, elapsed `faint`.

`docs/design/loam.md` records these choices in prose, so the theme can be judged against a written design, as Ultraviolet is.

## Implementation steps

1. `data.rs` with the canned data (40 seeds with season and days-to-sprout; 6 beds; a week of weather; a harvest list; 5 journal entries; taken garden names).
2. `theme.rs` (tokens only), with unit tests: every Unicode glyph is one column wide; night and day differ on every role; the ASCII set has no non-ASCII character.
3. `main.rs`: flags, motion, `set_theme`, `set_options` (confirm buttons on, password mask on, live motion, `cancel_message`, unattended on `--yes`), dispatch.
4. `commands.rs`, one function per command, in the order above. Keep each under about 80 lines; a helper that only draws is a sign something belongs in paloma.
5. `tests/loam.rs` golden transcripts, like `tests/ultraviolet.rs`: every command with `--yes` (and the presets) in five setups: night truecolor, day 256, night 16, `NO_COLOR`, `--ascii`. Snapshots `tests/snapshots/loam-*.txt`.
6. Docs: `docs/design/loam.md`, USAGE, DEVELOPMENT, ROADMAP.

## Tests and acceptance criteria

- **No UI code:** `examples/loam/` has no `ui.rs`. Outside `theme.rs`, `grep` finds no `\x1b`, `console::Style`, `style(`, `measure_text_width`, `.bold()` or colour names.
- **Theme:** `theme.rs` implements only `tokens()`.
- **Existing output unchanged:** built-in theme snapshots and Ultraviolet transcripts are byte-identical (any new paloma token defaults to the old look).
- **Pty runs** (Python `pty` + `pyte`, helper at `/tmp/claude-0/-home-user-paloma/cdf05a7f-88e2-52b1-9414-d6b777cecf3c/scratchpad/ptycmp.py`), night truecolor, 100×30, each command driven to completion:
  - `init` with the `back_to` (choose 10 beds), `Esc` back from the companion step (skips the soil test), the conditional shade step, the delayed name check (type a taken name, pause, see the pending row then the warning);
  - `plant` with a fuzzy search and the disabled companion;
  - `harvest` with the confirm inside the running task;
  - `journal` with `↑` history and `Tab` completion;
  - `^C` in `init` exits 130 with the cancel message on its own line.

  Report the final screens in the Status notes.
- **Layouts:** `--plain` runs every `--yes` command with `SimpleTheme` and still reads well.
- **The full check set** from `DEVELOPMENT.md`.

## Out of scope

- Progress bars themselves (PAL-011): only the seams.
- Saving anything: Loam forgets everything when it exits.
- Changing Ultraviolet.

## Status notes

- 2026-10-11: Plan written; executor Sonnet 5.5, medium effort (client).
- 2026-10-11: Implemented on branch `claude/dazzling-galileo-dqa12c`, uncommitted (Sonnet 5.5, medium). New: `examples/loam/` (`main.rs`, `theme.rs`, `commands.rs`, `data.rs`; no `ui.rs`), `[[example]] loam` in `Cargo.toml` (`test = true`), `tests/loam.rs` with `tests/snapshots/loam-*.txt`, `docs/design/loam.md`; changed: `docs/USAGE.md`, `DEVELOPMENT.md`, `ROADMAP.md` (PAL-020 moved to Done). The commands are `init` (a `Flow`: name with the delayed check, zone, beds, sun, conditional shade, soil test, password), `plant`, `water`, `harvest`, `journal` (`--long` opens the editor), `status` (`--json`), `almanac` and `seeds`; with no command it prints the wordmark and runs `init`. `LoamTheme` implements only `tokens()` (`grep "fn format_" examples/loam/theme.rs` finds nothing). Outside `theme.rs`, `grep` finds no `\x1b`, `console::Style`, `style(`, `measure_text_width`, `.bold()` or colour names (only the words "rain" and "rain barrel" in copy).
- 2026-10-11, **paloma change**: one new token, `button_indent` (`PromptStyle`, `PromptLook`, default 0), written by `defaults::format_confirm_buttons` before the buttons. Reason: the confirm buttons sat at column 0 while every other row under a prompt (message row, legend, items) is indented 2 in Loam, and no token could move them (Ultraviolet overrides the hook for this). Default 0 keeps the built-in themes and Ultraviolet byte-identical (snapshots unchanged; Ultraviolet overrides `format_confirm_buttons`, so it never reads the token). Tests: the cascade in `tokens.rs` (`buttons_are_not_indented_unless_asked`) and the layout in `defaults.rs` (`buttons_start_after_the_button_indent`); `docs/USAGE.md` names it. No other token was missing.
- 2026-10-11, deviations, each with its reason:
  - **The copy is plain ASCII** (`...` for `…`, `-` for `–` and `·`, so the cancel line reads `put your trowel down - nothing was changed` and the polar zone is an item `polar` with the description `coming soon`). Reason: `--ascii` and `TERM=linux` must print ASCII from end to end, and the only way to keep one copy is to keep it ASCII; the theme's glyphs are what change between the two sets (the `loam-ascii.txt` transcript has no non-ASCII character).
  - **The step counter keeps `03/07`.** The plan asked for `3 of 7` only if a `step_format` token exists; it does not, so `step_style` is `subtle` and the counter keeps paloma's form. A format token would be needed for `3 of 7`.
  - **Six transcript setups, not five.** The five of the plan plus `loam-plain.txt` (`--plain`, `SimpleTheme`, `NO_COLOR`), which is the plan's "`--plain` runs every `--yes` command" criterion as a golden test. The command list also covers the refused presets (`--name` taken, `--zone polar`, `--zone nowhere`), the first run with no command, an unknown command and an unknown flag.
  - **`restyle` is not used.** `--plain` is `SimpleTheme` itself (the point is that the app has no look of its own), and the day palette is chosen when `LoamTheme` builds its tokens, so nothing needed to wrap a theme.
  - **The wordmark is a `section`** whose title is the `text::gradient` string (so it gets the section's blank line and `❦`), followed by a `description` message for the tagline. The gradient's end points come from `theme::wordmark(night)` (moss to honey), because colours may only appear in `theme.rs`.
  - **Hand-picked 256-colour indices** where the nearest index of two roles coincided (night `accent` and `muted` both map to 144, night and day `subtle` both to 101): night accent 107, strong 193, success 150, info 152; day accent 64, strong 22, subtle 102, error 131, warning 136. They are in the table of `docs/design/loam.md`.
  - **`--name` is validated by the app** (presets are not validated by paloma), and the name validator also refuses a taken name, while `on_change` (the fake 600 ms check, a warning when taken and a success row when free) only informs.
  - **`journal --long --yes` opens no editor** (it prints an info message): an unattended run has nobody to write, and the golden transcript must never launch `$EDITOR`.
  - **Seams take `args`** as well as the units (`sow(args, seeds)`, `water_beds(args, beds)`, `fetch_catalogue(args)`), because the faked waiting depends on motion. Each carries a `// PAL-011:`-style doc comment naming the progress call that replaces it (`water_beds` is the plan's `water(beds)`; `water` is the command's name).
  - **A `ctrlc` handler** (a dev-dependency already used by Ultraviolet) covers `Ctrl+C` while a spinner runs, where no prompt reads keys: it calls `terminal::restore()`, prints the same cancel message and exits 130. At a prompt paloma does it all (the `cancel_message` option).
  - **Extras:** `Sort` in `harvest` has `legend(true)`; `FuzzySelect` has no paging counter in paloma (it windows its list), so the `[Page 1/2]` counter shows on `water`'s `MultiSelect` on a short terminal; the final `Confirm` of `plant` uses `interact_opt`, so `Esc` there prints "nothing was sown"; `status` shows a "checking the sky..." task that ends with `clear()`.
- 2026-10-11, unused features, with reasons: `restyle` (above); `step_done` (the "which beds?" prompt uses its own `Done::Erase`, which the plan allows); `Options::flows`, `Flow::counter`, `esc_back` and `theme` (the defaults are what the app wants); `Select::max_length`, `FuzzySelect::vim_mode` / `prefill` / `max_length`, `Confirm::on_shortcut(false)`, `Password::allow_empty_password` / `mask_char` / `result_len` (paloma's defaults suit the app); `Input::answered_prompt` and `allow_empty`; `Link::text`, `Gradient::style` / `fallback`; `flow::printed` (the app prints only through paloma); per-component `stream` overrides; the `*_on` terminals; the `Task::theme` override. No progress bar: only the seams.
- 2026-10-11, checks, all on the final tree:
  - `cargo test --locked --all-features`: 120 library unit tests, `flows` 1, `loam` 6, `outcomes` 1, `themes` 1, `ultraviolet` 5, the Loam example's 8 unit tests, the Ultraviolet example's 8, 41 doctests (1 ignored as before). Default features: 114 unit tests, `flows` 1, `outcomes` 1, `themes` 1 (the `loam` and `ultraviolet` tests are feature-gated), 38 doctests. `--no-default-features`: 110 unit tests, the integration tests, 33 doctests. All pass.
  - `cargo lint`, `cargo clippy --locked --all-targets --no-default-features -- -D warnings`, `cargo format-check`, `cargo check --examples --all-features`, `RUSTDOCFLAGS=-Dwarnings cargo doc --locked --all-features --no-deps --document-private-items` and `cargo +1.85.0 check --locked --lib --all-features`: clean.
  - Snapshots: `git status` shows only new files in `tests/snapshots/` (`loam-*.txt`); the `theme-*.txt` and `ultraviolet-*.txt` files are untouched and their tests pass without `UPDATE_SNAPSHOTS`. `Cargo.lock` is unchanged.
  - Pty runs (Python `pty` + `pyte`, 100×30, `TERM=xterm-256color`, `COLORTERM=truecolor`, a helper of my own in the scratchpad that also reads the exit status; screens read after every key):
    - `init`: the name `south bed` shows `◦ asking the neighbours...` about 0.4 s after the last key and then `☂ south bed is taken` with a nested `tip  the neighbours use it already` (Enter keeps the text and the warning); a free name shows `✓ nobody else uses that name`. 10 beds, then the soil test (`⤷ pH 6.4`, spinner `◜ testing the soil...`) ends with `back_to`: the zone line stays and the screen reads `✿ how many beds?  03/06 (10) ~` with `☂ the soil can feed 8 beds this year` and `tip  try fewer beds, or add compost first` under it. `no` at the sun question adds the shade step (`05/07`); the weak password shows `☂ not strong enough yet` with `tip  at least 8 characters` and `tip  one digit`; a mismatching confirmation starts over with `✗ those two do not match`; `Esc` at the password skips the soil test and lands on the shade select, `Esc` again on the sun confirm (counter `04/07`, `no` remembered); the final screen was:
      ```
      ❀ new garden — 7 steps

      ❦ your garden
      name      the back plot
      zone      temperate
      beds      6
      sun       about 6 hours
      sharing   kept behind a password
      ✓ the back plot is ready
        tip  next: loam plant
      ```
    - `plant`: the catalogue task, then `✿ which seed? ~ ▏search seeds...`; `ro` filters to rosemary, carrot, beetroot, marigold; Down, Enter picks carrot; the companions show `◉ marigold`, `◉ nasturtium`, `◌ ...` with `pumpkin  out of season` skipped by Down (struck through), `a` toggles the enabled ones only, Enter with none checked shows `✗ choose at least one companion` above the legend `↑↓ move  space check  a all  enter submit`. `y` at `✿ sow them in bed 2? (y/n) ~ yes` runs the sowing task and prints the panel; `Esc` there prints `◦ nothing was sown`. Final screen of the sow run:
      ```
      ❀ sow them in bed 2? — yes
        ⤷ sowed tomato
        ⤷ sowed marigold
        ⤷ sowed nasturtium
      ✓ sowed 3 seeds  1.4s

      ·┈ sown ┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈·
      ┊ ✓ tomato in bed 2                   sprouts in 6-10 days ┊
      ┊ ✓ marigold in bed 2                  sprouts in 5-8 days ┊
      ┊ ✓ nasturtium in bed 2               sprouts in 7-12 days ┊
      ·┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈·
      ```
    - `water` on a 9-row terminal: `✿ which beds? ~ [Page 1/2]` with the dry beds 2 and 5 checked, the description and the legend under the items.
    - `harvest`: sort with Space and arrows, Enter; the task `◞ picking...` pauses for `✿ the zucchini is huge. pick it anyway?` with the buttons `‹yes›   no` (indented 2 by `button_indent`) and the spinner returns after the answer. With `yes`, the final screen was:
      ```
      ❀ the zucchini is huge. pick it anyway? — yes
        ⤷ picked zucchini
        ⤷ picked lettuce
        ⤷ picked basil
      ☂ picked 5 crops, the zucchini was overripe  2.4s
      tomatoes       1.2 kg
      runner beans   0.6 kg
      zucchini       2.4 kg
      lettuce        0.4 kg
      basil          0.1 kg
      in all         4.7 kg
      ```
      (with `no`: `⤷ left the zucchini on the vine`).
    - `journal`: Up twice shows `first runner bean flowers on the teepee`, Down twice returns to the placeholder; `water the bas` + Tab gives `water the basil`, ` and the thy` + Tab gives `water the basil and the thyme`; Enter leaves `✿ write a note ~ water the basil and the thyme`, `✓ noted` and `tip  loam forgets it when it closes`. The terminal cursor codes written were `ESC[5 q` (blinking bar) and `ESC[0 q` (reset). `journal --long` with `EDITOR="sh -c 'echo planted the peas >> \"$0\"'"` prints `✓ note written` and the text; with `EDITOR=true` it prints `☂ nothing was saved` with a tip.
    - `^C`: in `init` at the zone prompt (after the name answer) the screen is `❀ name your garden — the back plot` then `tip  put your trowel down - nothing was changed` on its own line, exit status 130, cursor visible; at the name prompt after typing `ab` only the tip line is left, exit 130; during the soil test spinner and during `plant`'s catalogue spinner (the `ctrlc` handler) the spinner line is gone, the tip is on its own line and the exit status is 130.
    - `status` (the spinner is cleared, then the key/value block, `☂ frost tonight` with its tip and `✗ the bean teepee has not been watered in 9 days` with `tip  run: loam water`), `almanac` (wordmark `❦ loam`, tagline, the dotted `this week` panel, the key/value block with the link) and `init`, `status` with `--plain`, `--day --ascii` and `TERM=linux` (the ASCII set, `<no>` as the focused button) read as expected.
- 2026-10-11, unsure: `☂` and `⤷` measure one column in `console` (a unit test checks all glyphs), but some fonts draw `☂` wider or as an emoji; the plan chose it, so it stays, and the ASCII set is the fallback. Colours were checked by cell colour in `pyte` (for example the prompt mark `#9dbf7a`, question `#ede3d1`, counter and suffix `#8c7e6a`) and in the golden transcripts, not by eye on a real terminal.
