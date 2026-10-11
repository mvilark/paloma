# PAL-008: Live tasks (spinners)

Roadmap item: [PAL-008 in ROADMAP.md](../../ROADMAP.md). Umbrella plan: [PAL-004, Phase 3](PAL-004-ui-framework.md#pal-008--phase-3-live-tasks). Builds on [PAL-006](PAL-006-foundations-and-messages.md) (tokens, `Context`, `Status`, options families, `settings!`) and [PAL-007](PAL-007-output-blocks.md) (component layout, `emit`).

## Executor

**Sonnet 5.5, high effort.** The design is closed below, so a stronger model is not needed. The risk is in the details: a background redraw thread, a hidden cursor that must come back on every exit path (finish, drop, panic, Ctrl+C), the motion-off path, and pausing redraws while a prompt is shown. Medium effort would likely miss one of those paths. The planner reviews the diff and reruns every check before pushing.

## Context and decisions

- paloma is a UI framework for CLI programs that run one action and exit; line-oriented, never full-screen (client, 2026-10-10).
- Not opinionated: every look is a token or a theme hook, every behaviour a setting with a sensible default; everything works out of the box with minimal code (client, 2026-10-10).
- Shared base, individual overrides: families share a base for settings and style, members override (client, 2026-10-10). Here: family `live`, member `task` (progress bars join the family in PAL-011).
- paloma's own small live engine, no `indicatif` (client, 2026-10-10). One live component at a time in this phase; several at once is PAL-013.
- paloma installs no signal handler; it offers `paloma::terminal::restore()` for an app's Ctrl+C handler (PAL-004 plan).
- A prompt shown while a task runs pauses the task's redraws and resumes them after the answer, so a step can ask something in the middle of work (PAL-004 plan, 2026-10-10).
- Builder naming: setters named after what they set, no `with_` / `_with` (client, 2026-10-10).

## Files

Create:
- `src/live/mod.rs`: `pub(crate) mod engine; pub(crate) mod task;` and a module doc.
- `src/live/engine.rs`: the live region (global active-region slot, redraw thread, pause/resume, restore).
- `src/live/task.rs`: `Task`, `RunningTask`, `TaskView`, `TaskStepView`, `TaskState`, `task()`.
- `examples/tasks.rs`.
- Snapshot files created by the tests: `tests/snapshots/theme-simple-tasks.txt`, `tests/snapshots/theme-colorful-tasks.txt`.

Change:
- `src/lib.rs`: `mod live;` and `pub use live::task::{task, RunningTask, Task};`.
- `src/theme/tokens.rs`: spinner frames and the nested-step glyph in `Glyphs`; `LiveStyle`, `LiveTokens`, `LiveLook`; `Tokens::live` / `get_live`; presets.
- `src/theme/mod.rs`: re-exports; hooks `format_task`, `format_task_step`.
- `src/theme/defaults.rs`: `format_task`, `format_task_step`.
- `src/theme/restyle.rs`: forward both hooks.
- `src/theme/render.rs`: `TermThemeRenderer` pauses the live region while it exists.
- `src/options.rs`: `LiveOptions`, groups `live` and `task`.
- `src/terminal.rs`: `pub fn restore()`.
- `tests/themes.rs`: a task sheet for both built-in themes, and the restyle equality check.
- `docs/USAGE.md`, `DEVELOPMENT.md`, `ROADMAP.md`, this file's status notes.

## Public API

```rust
// src/live/task.rs, re-exported at the crate root.

/// A task label shown with a spinner while work runs; see [`Task::start`].
pub fn task(label: impl Into<String>) -> Task;

#[derive(Clone)]
pub struct Task { /* label, theme: Option<Arc<dyn Theme + Send + Sync>>, stream: Option<Stream> */ }

impl Task {
    /// A task with this running label, such as "Installing dependencies...".
    pub fn new(label: impl Into<String>) -> Self;
    /// Draws this task with `theme` instead of the app theme. Takes the theme by value
    /// (not `&t` like other components) because the spinner redraws from another thread.
    pub fn theme(self, theme: impl Theme + Send + Sync + 'static) -> Self;
    /// Where the task draws. Left unset: options `task`, then `live`, then stderr.
    pub fn stream(self, stream: Stream) -> Self;
    /// Starts the task: with motion, a spinner line redrawn every frame; without
    /// motion, nothing yet. Fails if another live component is running.
    pub fn start(self) -> Result<RunningTask>;
    /// Like `start` but draws on `term` (motion is off unless `term` is a terminal).
    pub fn start_on(self, term: &Term) -> Result<RunningTask>;
}

/// A started task. Dropping it without finishing clears the spinner line and restores
/// the cursor, printing nothing.
pub struct RunningTask { /* ... */ }

impl RunningTask {
    /// Prints a nested line under the running task (the spinner stays below it).
    pub fn step(&self, text: impl Into<String>) -> Result;
    /// Changes the running label.
    pub fn label(&self, label: impl Into<String>) -> Result;
    /// Time since `start`.
    pub fn elapsed(&self) -> Duration;
    /// Stops the spinner and prints the result line for `status` with `text`;
    /// returns the elapsed time.
    pub fn finish(self, status: Status, text: impl Into<String>) -> Result<Duration>;
    /// `finish(Status::Success, text)`.
    pub fn success(self, text: impl Into<String>) -> Result<Duration>;
    /// `finish(Status::Warning, text)`.
    pub fn warning(self, text: impl Into<String>) -> Result<Duration>;
    /// `finish(Status::Error, text)`.
    pub fn fail(self, text: impl Into<String>) -> Result<Duration>;
    /// Stops the spinner and erases its line, printing no result; step lines stay.
    pub fn clear(self) -> Result<Duration>;
}

// src/live/task.rs, re-exported from `paloma::theme`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TaskState { Running, Finished(Status) }

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct TaskView<'a> {
    pub text: &'a str,       // the running label, or the result text once finished
    pub state: TaskState,
    pub elapsed: Duration,
    pub frame: usize,        // spinner frame counter (the hook takes it modulo the frame count)
}
impl<'a> TaskView<'a> { pub fn new(text: &'a str, state: TaskState, elapsed: Duration, frame: usize) -> Self; }

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct TaskStepView<'a> { pub text: &'a str }
impl<'a> TaskStepView<'a> { pub fn new(text: &'a str) -> Self; }

// src/theme/mod.rs, new Theme hooks (defaults in theme::defaults, forwarded by Restyled).
fn format_task(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &TaskView) -> fmt::Result;
fn format_task_step(&self, f: &mut dyn fmt::Write, ctx: &Context, view: &TaskStepView) -> fmt::Result;

// src/terminal.rs
/// Stops any running live component, erases its line and shows the cursor again.
/// Safe to call from a Ctrl+C handler, and when nothing is running.
pub fn restore();

// src/options.rs
settings! {
    /// Settings for live components: the `live` family group and the `task` group.
    pub struct LiveOptions {
        /// Whether live components redraw (spinners turn). Default: on when the stream
        /// is a terminal and `CI` is not set. Never on when the stream is not a terminal.
        motion: bool,
        /// Milliseconds between frames. Default 80.
        frame_ms: u64,
        /// Where live components draw. Default stderr.
        stream: Stream,
    }
}
// Options::live(|l| …) and Options::task(|t| …); resolution: task group, live group, default.

// src/theme/tokens.rs
// Glyphs: `nested` (&'static str; Unicode "↳", ASCII "-", none "") and
// `spinner: &'static [&'static str]` (Unicode ["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"],
// ASCII ["-","\\","|","/"], none []), with setters `nested(..)` / `spinner(..)` and a getter
// `get_spinner() -> &'static [&'static str]`; `GlyphName::Nested` for `get`.
settings! {
    pub struct LiveStyle {
        spinner_style: Style,   // default role accent
        label_style: Style,     // default role muted
        step_style: Style,      // default role muted
        nested_style: Style,    // default role faint (the ↳ glyph)
        elapsed_style: Style,   // default role subtle
        step_indent: usize,     // default 4
        show_elapsed: bool,     // default true
    }
}
// LiveTokens { base, task } with closure setters `base(|s| …)`, `task(|s| …)`;
// `LiveTokens::resolve(LiveKind::Task, &Tokens) -> LiveLook` (all fields concrete);
// `pub enum LiveKind { Task }` (#[non_exhaustive]); Tokens::live(..) / get_live().
```

Default drawing (`theme::defaults`):
- Running: `{frame glyph} {label}` then, if `show_elapsed`, two spaces and the elapsed time as `{secs:.1}s`. The frame glyph is `spinner[frame % len]` in `spinner_style`; with no spinner frames, no glyph and no space.
- Finished: the result line is drawn like a message of that status (reuse `tokens.get_messages().resolve(status, tokens)`: glyph, word, text styles, exactly as `defaults::format_message` does at depth 0), then, if `show_elapsed`, two spaces and the elapsed time in `elapsed_style`.
- Step: `step_indent` spaces, the `nested` glyph in `nested_style` and a space (both skipped when the glyph is empty), then the text in `step_style`.

So `SimpleTheme` prints `success: Created your profile  0.0s`, steps as `    - Generated device key`, and its spinner as ASCII frames.

## Implementation steps

1. **Tokens.** In `src/theme/tokens.rs`: add `nested` to the `glyphs!` list (and to `unicode()`, `ascii()`, `ascii_frames()`), add the `spinner` field by hand next to the macro-generated struct (the macro only handles `&'static str` fields; extend it or add the field, setter and getter manually, keeping `Default` working), and `GlyphName::Nested`. Add `LiveStyle` with `settings!` (see `MessageStyle` and `BlockStyle` in the same file for the pattern), `LiveKind`, `LiveTokens` with a `live_groups!` macro like `block_groups!`, `LiveLook`, `resolve`, and the `live` field on `Tokens` with `live()` / `get_live()`. Both presets get `LiveTokens::default()`; the colorful preset needs nothing else (defaults come from the palette).
2. **Views and hooks.** Put `TaskState`, `TaskView`, `TaskStepView` in `src/live/task.rs` and re-export them from `src/theme/mod.rs` (as `SectionView` is). Add `format_task` / `format_task_step` to `Theme` (doc comments like `format_section`), their defaults in `src/theme/defaults.rs`, and both to the `forward!` list in `src/theme/restyle.rs` (add the imports there).
3. **Options.** In `src/options.rs`: `LiveOptions` via `settings!`, fields `live` and `task` on `Options`, both in the `groups!` list, and `pub(crate) fn live_member(&self, kind: LiveKind) -> &LiveOptions`.
4. **Engine** (`src/live/engine.rs`):
   - `static ACTIVE: Mutex<Option<Arc<Region>>>` (use `unwrap_or_else(|e| e.into_inner())` on poisoned locks, as `options.rs` does).
   - `Region` holds: the `Term`, the stream, `Arc<dyn Theme + Send + Sync>`, owned `Tokens` (from `theme.tokens().into_owned()`), a `Mutex<Frame>` (text, state, start `Instant`, frame counter), `AtomicBool stop`, `AtomicBool paused`, `motion: bool`, `frame_ms: u64`, and the thread `JoinHandle` in a `Mutex<Option<…>>`.
   - `Region::start(...) -> Result<Arc<Region>>`: fails with `io::Error::other("another live component is running")` if `ACTIVE` is taken; with motion, hides the cursor, draws the first frame and spawns the redraw thread; registers itself in `ACTIVE`.
   - Redraw thread: loop until `stop`; every `frame_ms` ms, unless `paused`, lock the frame, `term.clear_line()`, write the running line (no newline), flush, bump the counter. All writes to the term happen while holding the frame lock, so steps and frames never interleave.
   - `step(text)`: lock; with motion, `clear_line` first; `write_line` the step; with motion, redraw the running line right away.
   - `finish(state)`: set `stop`, join the thread, lock; with motion, `clear_line`; write the result line (`write_line`) unless clearing; show the cursor; free `ACTIVE`.
   - `pause()` / `resume()`: crate-private functions on the active region, if any. Pause: set `paused`, lock, `clear_line`, flush (the prompt draws where the spinner was). Resume: clear `paused`; the next tick redraws below whatever the prompt left. Expose them through a guard type `pub(crate) struct Pause` (pause in `Pause::new()`, resume in `Drop`).
   - `restore()`: if a region is active, set `stop` (do not join: it may be called from a signal-handler thread), lock briefly, `clear_line`, show the cursor, free `ACTIVE`. Always also `Term::stderr().show_cursor()`. Re-export it from `src/terminal.rs`.
5. **Task** (`src/live/task.rs`): builder as in the API; `start_on` resolves theme (own `Arc` or `crate::theme::app::current()`), stream (own, then `options.task.stream`, `options.live.stream`, default stderr), motion (`term.is_term()` AND own-options `task.motion.or(live.motion).unwrap_or(std::env::var_os("CI").is_none())`), frame_ms (`task`, `live`, default 80). `RunningTask` holds the `Arc<Region>` and a `finished` flag; `Drop` calls the region's clear path when not finished. Without motion, `start` prints nothing, `step` prints its line, `finish` prints the result line.
6. **Prompts pause tasks.** Add a `_pause: crate::live::engine::Pause` field to `TermThemeRenderer` in `src/theme/render.rs`, created in `TermThemeRenderer::new`. Every prompt creates exactly one renderer per interaction, so the task pauses for the prompt and resumes when it returns.
7. **Example** `examples/tasks.rs`: install `ctrlc::set_handler(|| { paloma::terminal::restore(); std::process::exit(130) })` (`ctrlc` is already a dev-dependency); run a task with two steps and `success`; a task that ends with `warning`; a task that asks a `Confirm` in the middle and then `fail`s; use `std::thread::sleep` for fake work.
8. **Docs.** `docs/USAGE.md`: a "Tasks" section (API, motion rules, `terminal::restore` for Ctrl+C, a prompt pauses a task, tokens `live`). `DEVELOPMENT.md`: layout line for `src/live/`. `ROADMAP.md`: PAL-008 to Done with this plan's link. Status note here.

## Tests and acceptance criteria

- **Unchanged:** every existing file in `tests/snapshots/` must stay byte-identical (`cargo test --all-features`, no `UPDATE_SNAPSHOTS`). Adding the `nested` and `spinner` glyphs must not change message or block output.
- **New snapshots** in `tests/themes.rs`: a `task_sheet(theme)` like `block_sheet`, with lines for `format_task` running at frames 0 and 3 with `elapsed = 1.25s`, finished with each status, a running task with `show_elapsed(false)` via `restyle`, and `format_task_step`; asserted for `SimpleTheme` and `ColorfulTheme`; and `assert_eq!(task_sheet(&restyled), task_sheet(&colorful))`.
- **Unit tests** in `src/live/task.rs`: drive `start_on` with `Term::read_write_pair` over a shared buffer (a small `struct Buf(Arc<Mutex<Vec<u8>>>)` implementing `Read` (returning 0) and `Write`); motion is off because it is not a terminal. Assert the exact output of `start`, `step("a")`, `step("b")`, `success("done")` with `SimpleTheme` (elapsed shows `0.0s`), that `clear()` prints no result line, and that a second `start` while one runs returns an error. These tests touch the global `ACTIVE` slot: put them in one test function.
- **Pty check** (as in `DEVELOPMENT.md`, Python `pty` + `pyte`): run `examples/tasks.rs` at 30 rows; sample the screen during the first task (a braille frame is visible and changes between samples), at the end (step lines and result lines present, no stray spinner line), and check `screen.cursor.hidden` is false at the end. Send `n` at the confirm and check the third task resumes below the answered prompt. Run once more with `CI=1` (no frames; each result line printed once). Send Ctrl+C during a spinner and check the cursor is visible and the exit code is 130.
- **Full check set:** `cargo test --locked` with all, default and no features; `cargo lint`; `cargo format-check`; `cargo check --examples --all-features`; `RUSTDOCFLAGS=-Dwarnings cargo doc --locked --all-features --no-deps --document-private-items`; `cargo +1.85.0 check --locked --lib --all-features`.

## Out of scope

Progress bars (PAL-011), several live lines at once (PAL-013), moving the Ultraviolet example onto tasks (after all phases), a cursor guard for prompts (prompts keep managing their own cursor).

## Status notes

- 2026-10-11: Plan written. Executor: Sonnet 5.5, high effort; the planner reviews before push.
