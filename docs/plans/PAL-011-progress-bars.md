# PAL-011: Progress bars

The last phase of the [PAL-004](PAL-004-ui-framework.md) umbrella plan (Phase 7). It adds `Progress`, a live progress bar on the same live region as `Task`. The Ultraviolet example then drops its hand-drawn bar, and with it `ui.rs`. Loam's three progress seams switch to real bars.

## Executor

**Sonnet 5.5, high effort.**

- **Design left open:** none. The API, tokens, hook, view, defaults, layout and example changes are fixed below.
- **Risk:** high in the engine.
  - The live region (`src/live/engine.rs`) is written for `Task` only and must learn a second kind of frame. It has a redraw thread, pausing while a prompt shows, a hidden cursor that must come back on every path, and a test lock (`TEST_REGION`).
  - `Task`'s output must stay byte-identical.
  - Ultraviolet's transcripts must stay byte-identical, so the bar's motion-off line must reproduce the example's spans exactly.
  - This is the same profile as PAL-008 (Sonnet high, no rework).
- **Size:** moderate: one component, an engine refactor, tokens, a hook, options, tests, two example migrations and docs.
- **Verification:** golden transcripts, unit tests and pty runs of the animation.

The planner reviews the diff and reruns the checks before anything is pushed. The executor leaves its changes uncommitted, and waits on long checks in the foreground or on a specific PID (never `pgrep -f` on a pattern in its own command line; see TEMPLATE.md).

## Context and decisions

- **PAL-004, Phase 7** (agreed 2026-10-10): "`Progress` on `LiveBase`: a total, `set` / `inc`, a label, a percentage, easing toward the real value (never backwards) and the head glint as live tokens. Motion off prints the finished line once. Example: `ui::progress` goes, and with it `ui.rs`."
- **Direction decisions** (PAL-004, 2026-10-10):
  - paloma has its own live engine, with no `indicatif`.
  - One live region at a time; several at once is PAL-013, an idea, not planned.
  - paloma is line-oriented.
- **Ultraviolet's final output must not change.** Its transcripts (`tests/snapshots/ultraviolet-*.txt`) stay byte-identical. These runs have no motion, so they hold the bar's finished line.
- **paloma is not opinionated** (PAL-004 status note, 2026-10-10). Everything about the bar's look is a token, the layout is a hook, and defaults are sensible with no setup.
- **Loam's three seams** (PAL-020) were left for this phase: `sow`, `water_beds` and `fetch_catalogue` in `examples/loam/commands.rs`, each marked `// PAL-011:`.
- **Lull is unchanged.** It shows nothing that takes time.

What Ultraviolet draws today (`examples/ultraviolet/ui.rs`, `progress`):
- **The bar:** 32 cells of the theme's `bar` glyph (`━`, or `=` in ASCII). The filled part is in `accent`; the last 2 filled cells (the glint) are in `accent-strong` (`Tok::Strong`); the rest, the track, is in `faint`.
- **The rest of the line:** a space, the label in `muted`, two spaces, and the integer percentage in `subtle` (`{cell*100/32}%`).
- **The finished line:** all 32 cells in `accent` (one styled span), then `" "`, the label span, `"  "` and the `100%` span.
- **Without motion,** only the finished line is printed. That line is what the snapshots hold.

## Public API

In `src/live/progress.rs`, exported from `lib.rs` beside `Task`:

```rust
/// A progress bar for `label`, out of `total` units. Start it with `start`.
pub fn progress(label: impl Into<String>, total: u64) -> Progress;

pub struct Progress { /* label, total, theme, stream */ }
impl Progress {
    pub fn new(label: impl Into<String>, total: u64) -> Self;
    /// Draws with this theme instead of the app's.
    pub fn theme(self, theme: impl Theme + Send + Sync + 'static) -> Self;
    /// Draws on this stream instead of the `live` options' one.
    pub fn stream(self, stream: Stream) -> Self;
    /// Shows the bar at 0. Fails if another live component is running.
    pub fn start(self) -> Result<RunningProgress>;
    /// The same on a given terminal (tests).
    pub fn start_on(self, term: &Term) -> Result<RunningProgress>;
}

pub struct RunningProgress { /* … */ }
impl RunningProgress {
    /// Sets the value; it is clamped to `total`. A value below the current one is ignored,
    /// so the bar never moves backwards.
    pub fn set(&self, value: u64) -> Result;
    /// Adds `by` to the value, clamped to `total`.
    pub fn inc(&self, by: u64) -> Result;
    /// Changes the label.
    pub fn label(&self, label: impl Into<String>) -> Result;
    /// The current value (not the eased one on screen).
    pub fn value(&self) -> u64;
    pub fn elapsed(&self) -> Duration;
    /// Leaves the bar on screen as a lasting line, at the current value, with no glint.
    pub fn keep(self) -> Result<Duration>;
    /// Replaces the bar with a result line, drawn like a task's.
    pub fn finish(self, status: Status, text: impl Into<String>) -> Result<Duration>;
    pub fn success(self, text: impl Into<String>) -> Result<Duration>;
    pub fn warning(self, text: impl Into<String>) -> Result<Duration>;
    pub fn fail(self, text: impl Into<String>) -> Result<Duration>;
    /// Removes the bar and leaves nothing.
    pub fn clear(self) -> Result<Duration>;
}
// Drop without finishing: like `clear` (the cursor comes back), as `RunningTask` does.
```

- **`total == 0`** draws an empty track with `0%`, and `keep` then shows 100% (a finished job of nothing is done).
- **Percentage:** `value * 100 / total`, an integer, as Ultraviolet computes it.
- **Steps:** `RunningProgress` has no `step`, on purpose. A bar's label carries what is happening; a task is for step lines.

### The view and the hook

In `src/live/progress.rs`, re-exported from `paloma::theme` like `TaskView`:

```rust
#[non_exhaustive]
pub enum ProgressState { Running, Kept, Finished(Status) }

#[non_exhaustive]
pub struct ProgressView<'a> {
    /// The label; for `Finished`, the result text.
    pub text: &'a str,
    pub state: ProgressState,
    /// The value drawn: the eased one while running, the real one when kept.
    pub value: u64,
    pub total: u64,
    pub elapsed: Duration,
}
impl<'a> ProgressView<'a> { pub fn new(text, state, value, total, elapsed) -> Self; }
```

New `Theme` hook, `fn format_progress(&self, f, ctx, view: &ProgressView) -> fmt::Result`, with its default body `defaults::format_progress`. `Restyled` must forward it (`src/theme/restyle.rs` forwards every method; there is a test or a pattern for that, so follow it). The default layout:
- **`Running` and `Kept`:**
  - `bar_indent` spaces, then the bar.
  - The bar is `cells = value * bar_width / total` (integer, `bar_width` when `total == 0` and kept, else 0) filled cells and `bar_width - cells` track cells.
  - While `Running`, the last `min(head_width, cells)` filled cells are the head.
  - Then `" "`, the label in `label_style`, and, if `show_percent`, `"  "` and the percentage in `percent_style`.
  - If `show_elapsed`, `"  "` and the elapsed time as the task draws it.
- **Painting:** each part is painted as **one span**: the fill without the head, the head, the track, the label and the percentage. An empty part is not painted, so it writes nothing (no empty escape pair). This is what makes Ultraviolet's line byte-identical.
- **`Finished(status)`:** exactly `format_task`'s finished line: `format_message` of that status and text, plus the elapsed time if `show_elapsed`.

### Tokens

`LiveKind` gains `Progress`. `LiveTokens` gains a `progress` group (via the existing `live_groups!` macro). `LiveStyle` (the `settings!` struct in `src/theme/tokens.rs`) gains these fields, resolved into `LiveLook`:

| Token | Type | Default |
| --- | --- | --- |
| `bar_width` | `usize` | 30 |
| `bar_indent` | `usize` | 0 |
| `head_width` | `usize` | 0 (no glint) |
| `fill_style` | `Style` | `accent` |
| `head_style` | `Style` | `accent_strong` |
| `track_style` | `Style` | `faint` |
| `percent_style` | `Style` | `subtle` |
| `show_percent` | `bool` | true |

- **`show_elapsed` defaults per kind:** true for `Task` (unchanged) and false for `Progress`. Resolve with `s.show_elapsed.unwrap_or(kind == LiveKind::Task)`.
- **`label_style`** already exists (default `muted`) and serves both kinds.

New glyphs, `GlyphName::BarFill` and `GlyphName::BarTrack`, with builder methods `bar_fill` / `bar_track`:
- `Glyphs::none()` (SimpleTheme): `=` and `-`.
- `Glyphs::unicode()` (ColorfulTheme): `█` and `░`.
- **No colour:** the fill and the track must differ, because without colour (`NO_COLOR`, SimpleTheme) nothing else tells them apart.

### Options

`Options::live(|l| l.progress(|p| …))`: a `progress` member group of the `live` family, next to `task`, with the same `LiveOptions` fields. The member's `frame_ms` default is **40** (smooth easing); the task keeps 80. Motion follows the same rule as tasks.

### Easing

The redraw thread keeps a `shown` value (`f64`). On every frame, while `shown < value`:
- `shown += max((value - shown) * 0.25, total / bar_width / 4)`, capped at `value`;
- with `total == 0`, there is nothing to ease.

`shown` never decreases. The view's `value` while running is `shown.floor() as u64`. `keep` and `finish` stop the thread first, then draw with the real value. `set` and `inc` only store the value and wake the thread, so a burst of calls costs nothing. With motion off they draw nothing.

## Files

- **`src/live/progress.rs`** (new): `progress`, `Progress`, `RunningProgress`, `ProgressView`, `ProgressState`, and unit tests on a recording `Term`, as `task.rs` does.
- **`src/live/engine.rs`:** the region draws either kind. `Frame` holds a `Body` enum (`Task { label, counter }`, `Progress { label, value, shown, total }`). `draw` dispatches to `format_task` / `format_progress`; `run` advances the spinner counter or the easing. Spec gains the body.
  - The finish path takes an enum of what to leave: nothing, a result `(Status, text)`, or the kept bar.
  - Keep `step` task-only.
  - `TEST_REGION` handling is unchanged: progress tests take the write lock like the task test.
- **`src/live/task.rs`:** adapt to the engine's new `Spec`/`finish` signatures. No behaviour change.
- **`src/live/mod.rs`:** the new module.
- **`src/lib.rs`:** exports.
- **`src/theme/tokens.rs`:** `LiveKind::Progress`, the `progress` group, the `LiveStyle`/`LiveLook` fields, the glyphs, and the defaults above.
- **`src/theme/mod.rs`:** the hook and the re-exports.
- **`src/theme/defaults.rs`:** `format_progress`.
- **`src/theme/restyle.rs`:** forwarding.
- **`src/options.rs`:** the `progress` member and its `frame_ms` default.
- **`examples/progress.rs`** (new, like `examples/tasks.rs`): a download-like bar driven by `inc`, a bar that is kept, one finished with `success`, and a prompt shown mid-bar (the bar pauses).
- **`tests/themes.rs`:** add `format_progress` to the golden hook output of the built-in themes. This adds lines only; the existing ones stay identical.
- **The examples, Ultraviolet and Loam:** see below.
- **Docs:**
  - `docs/USAGE.md`: a "Progress bars" section next to "Tasks", with the tokens table.
  - `DEVELOPMENT.md`: the module list and "Ultraviolet has no `ui.rs`".
  - `ROADMAP.md`: PAL-011 and PAL-004 move to Done.
  - The status notes of this plan and of PAL-004.

## Implementation steps

1. **Engine first,** with no new behaviour: introduce `Body` and the finish enum, port `Task` onto them, and run `cargo test --all-features`. The task unit tests, `tests/ultraviolet.rs` and `tests/loam.rs` must pass unchanged before going on.
2. **Tokens, glyphs, view, hook and default body,** with unit tests of `format_progress` in `defaults.rs`:
   - running with a head;
   - the head longer than the fill;
   - kept at 50% and at 100%;
   - `total == 0`;
   - no `show_percent`;
   - `show_elapsed`;
   - finished;
   - the one-span rule: assert the exact escape sequence with a forced-colour context (as other `defaults.rs` tests do).
3. **`Progress`, `RunningProgress`, options and easing,** with unit tests on the recording `Term`:
   - motion off: `keep` prints exactly one line, `success` exactly one line, `clear` nothing;
   - `set` lower is ignored, and `inc` clamps;
   - a second live component while one runs fails;
   - drop restores the cursor;
   - easing reaches the value and never goes backwards. Test the pure step function: make it a private `fn ease(shown, value, total, width) -> f64`.
4. **`examples/progress.rs`.**
5. **Ultraviolet:**
   - **Tokens:** in `theme.rs`, set the progress tokens: `bar_width(32)`, `head_width(2)`, `fill_style` accent, `head_style` strong, `track_style` faint, `label_style` muted and `percent_style` subtle. Set the glyphs `bar_fill` and `bar_track` both to the theme's `bar` glyph (`━`, `=` in ASCII). Use the theme's own style helpers, exactly as for the task tokens.
   - **`flows.rs`:** replace `ui::progress("Exchanging keys")` with `let bar = paloma::progress("Exchanging keys", 32).start()?; for _ in 0..32 { work(…); bar.inc(1)?; } bar.keep()?;`. The pauses keep the old timing: 45 ms + `cell * 40 / 32` ms each.
   - **`ui.rs` goes:**
     - `work` moves to `flows.rs`, where it is used.
     - `wordmark` and `wordmark_with_icon` move to `theme.rs` (the Serrallonga icon and wordmark are the design system's brand art, as PAL-004's end state says: "What stays in the example: … the wordmark text and the Serrallonga icon art").
     - Update the module docs of `main.rs`, `theme.rs` and `flows.rs`, and `mod ui` goes.
   - **Check:** `tests/ultraviolet.rs` must pass with **no snapshot change**. If a byte differs, fix the default hook or the tokens, never the snapshot.
6. **Loam:** the three seams become progress bars. Remove the `// PAL-011:` markers, and update their doc comments to say what they show.
   - **`sow(args, seeds)`:** `progress("sowing...", seeds.len())`. Before each seed's pause, `label(format!("sowing {seed}..."))`, then `inc(1)`. Finish with `success(format!("sowed {}", count(...)))`.
   - **`water_beds(args, beds)`:** a bar over the total litres, labelled `watering {bed}...` per bed, `inc(bed.litres)` per bed, then the same success line as today.
   - **`fetch_catalogue(args)`:** a bar over `SEEDS.len()`, labelled `fetching the seed catalogue...`, with `inc(10)` per pause, then `success` as today.
   - **Theme tokens** in `examples/loam/theme.rs`:
     - `bar_width(24)` and `head_width(1)`;
     - fill `Accent`, head `Strong`, track `Faint`, label `Muted`, percent `Subtle`;
     - glyphs `bar_fill` `━` and `bar_track` `┈`, or `=` and `-` in ASCII.
     - Add them to the `Set` struct and its tests (one column, ASCII set is ASCII).
   - **Snapshots:** the Loam transcripts **change**, intentionally. The step lines under those three tasks disappear, and the success lines stay. Regenerate with `UPDATE_SNAPSHOTS=1`, check that the diff is only those lines, and list it in the status notes.
   - **Loam's `ctrlc` handler** stays: it also covers a bar.
7. **Docs and the full check set.**

## Tests and acceptance criteria

- **Byte-identical:** `tests/snapshots/theme-*.txt`, apart from the added `format_progress` lines; every `ultraviolet-*.txt`; every `lull-*.txt`. Loam changes only as in step 6.
- **Unit tests** as listed in steps 2 and 3. The Ultraviolet and Loam theme tests are updated for the new glyphs.
- **Pty checks** (Python `pty` + `pyte`, as in DEVELOPMENT.md). Record what the screens show in the status notes.
  - `cargo run --example progress`:
    - frames advance smoothly; take several screenshots during the run and check that the fill only grows;
    - the cursor is hidden while running and visible after;
    - the kept line stays, and the success line replaces its bar;
    - the prompt mid-bar pauses it, and it resumes under the answer.
  - Ultraviolet `connect` with motion (no `CI`):
    - the bar fills over about 2 s with the glint at the head;
    - the final screen equals the motion-off line;
    - compare it cell by cell (text and colours) with the same run on `main` (build `main` in a separate worktree in the scratchpad).
  - Loam `plant` with motion: the bar labelled `sowing tomato...` and so on, then `✓ sowed 3 seeds`.
  - `^C` during the Ultraviolet bar: the cursor comes back and the exit code is 130, the same as during a task.
  - `CI=1`: no animation, one line.
- **Full check set** from DEVELOPMENT.md:
  - `cargo test --locked` with all, default and no features;
  - `cargo lint` and `cargo clippy --locked --all-targets --no-default-features -- -D warnings`;
  - `cargo format-check` and `cargo check --examples --all-features`;
  - `RUSTDOCFLAGS=-Dwarnings cargo doc --locked --all-features --no-deps --document-private-items`;
  - `cargo +1.85.0 check --locked --lib --all-features`;
  - cross-target `cargo check --lib --all-features --target x86_64-pc-windows-msvc` and `--target x86_64-apple-darwin` (both targets are installed).
- **Flakiness:** run the progress and task unit tests 30 times in a loop (`for i in $(seq 30); do cargo test -q --all-features --lib live || break; done`). Every run must pass.

## Out of scope

- **Several bars or tasks at once** (PAL-013, an idea).
- **Indeterminate bars:** a task's spinner covers unknown totals.
- **Rate and time left (ETA):** a theme can compute them from `elapsed`, `value` and `total` in its own `format_progress`.
- **Byte or size formatting.**
- **Lull,** and the older small examples (their modernisation is not approved).

## Status notes

- 2026-10-13: Plan written. Executor: Sonnet 5.5, high effort. The client asked (2026-10-13) to "keep working according to the plans, until all that is planned is done", using agents with care for model usage. This is the last planned phase; with it PAL-004 is complete.
