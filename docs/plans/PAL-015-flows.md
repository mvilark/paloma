# PAL-015: Flows

Roadmap item: [PAL-015 in ROADMAP.md](../../ROADMAP.md). Umbrella plan: [PAL-004, Phase 6](PAL-004-ui-framework.md#pal-015--phase-6-flows). Builds on PAL-009 (views, tokens), PAL-010 (frames, message row), PAL-017 (`Done`, back and cancel, presets, unattended mode), PAL-018 and PAL-019. Read their Public API sections and Status notes first: they say how each phase was actually built.

## Executor

**Opus 5.5, medium effort** (the client's choice, 2026-10-11; first assessed as high).

| Criterion | Assessment |
| --- | --- |
| Design left open | Closed here, but the flow is the first component that coordinates the others (prompts, messages, blocks, tasks), through global state. |
| Risk of subtle bugs | High: counting the screen rows a step printed (wrapping, both streams), erasing and redrawing them on back and `keep_last`, `Esc` semantics in every prompt, `^C` inside nested calls. |
| Size | Medium to large: one new module, a recording hook in five output paths, a field on four views, the Ultraviolet setup. |
| Verification | Pty runs with `Esc`, `back_to` and `keep_last`, screen by screen; Ultraviolet transcripts unchanged. |

The design is closed here, so medium effort suffices for implementation; the row bookkeeping is covered by the pty checks below. The planner reviews the diff and reruns every check before pushing.

## Context and decisions

Client decisions (2026-10-10, umbrella plan):

- A flow runs the steps of one task in order. "A step is plain app code that usually asks one or more prompts, and can do anything else in between (call a server, show a spinner, print a message)." Steps must allow app code between prompts (for example a sign-in after the password).
- **Step counter:** "the total grows when it becomes known (`01/05`, then `04/06`)" and "can be turned off, per flow or app-wide in `Options`".
- **Esc:** "`Esc` in a prompt goes back one step. On the first step it cancels the flow (both are options with these defaults)."
- **`back_to`:** "one call jumps back to any earlier step: `return flow::back_to("email", Message::error("Wrong email or password"));`, or `flow::back_to("email", None)`. The message shows in the target prompt's message row."
- **Going back erases** "what the steps in between printed, by default, so the screen never shows an answer twice. The state keeps earlier values, so a step can prefill its prompt with them."
- **`.step_done(Done)`** for every prompt in the flow unless a prompt sets its own `done`; **`.keep_last(n)`** keeps only the last `n` answered steps visible while the flow runs; **`.done(Done)`** for the whole flow: everything (`Keep`), one summary line (`Collapse`) or nothing (`Erase`).
- **`^C`** cancels the whole flow: the theme draws the cancelled state and `run()` returns `Error::Cancelled`.
- **Unattended:** presets and unattended mode work per step.
- paloma is not opinionated: every behaviour has a default and a setting, every look goes through the theme.

Decided here:

1. **A flow records what its steps print.** paloma cannot read the screen back, so while a flow runs, each place where paloma writes a lasting line records it on the flow's ledger, tagged with the current step: answered lines and kept prompts, messages and blocks, a task's step and result lines. The ledger keeps the text, so the flow can count its screen rows when it erases, and print it again for `keep_last`.
   - Output written past paloma (an app's own `println!`) is not recorded. Inside a flow, apps print through paloma, or call `flow::printed(text)` after printing (documented).
   - Only output to a stream that is a terminal is recorded; a redirected stream never shows on screen.
2. **`Esc` inside a flow makes `interact()` return `Err(Error::Back)`**, so steps go back with plain `?`. Outside a flow, `interact()` ignores `Esc` as today. `interact_opt()` keeps returning `Ok(None)` everywhere, for apps that handle `Esc` themselves.
3. **Going back always erases** the target step's output and everything after it. The umbrella's "by default" gets no switch: nothing asked for keeping a stale answer, and it can be added later without breaking anything.
4. **`Esc` goes back to the nearest earlier step that asked something.** Steps answered by a preset or by unattended mode ask nothing, and going back into them would only bounce forward again. With no such step, `Esc` acts as on the first step.
5. **Counter numbering:** a step's number is its position among the steps that run, and the total counts the unconditional steps plus the conditional steps whose condition holds for the current state. Both are evaluated each time a step starts.
6. **The counter is drawn by the theme** from a new `step` field on the question views. The answered line has no counter (as Ultraviolet draws it today).
7. **One flow at a time.** `run()` while another flow runs returns an `io::ErrorKind::InvalidInput` error.

## Files

- `src/flow.rs` (new, public module `paloma::flow`): `Flow`, `back_to`, `printed`, and the crate-private ledger and active-flow state.
- `src/lib.rs`: `pub mod flow;`, re-export `Flow` at the root.
- `src/error.rs`: `Error::Back` and `Error::BackTo(BackTo)`.
- `src/options.rs`: `FlowOptions` and the `flows` group.
- `src/prompts/*.rs`, `src/prompts/base.rs`:
  - `interact` honours `Esc` inside a flow;
  - the `done` cascade includes the flow's `step_done`;
  - a prompt opening in a flow takes a pending `back_to` message into its row;
  - the prompt marks that its step asked something.
- `src/theme/render.rs`, `src/output/block.rs`, `src/live/engine.rs`: call the recording hook where lasting lines are written.
- `src/theme/prompt.rs`: `step: Option<(usize, usize)>` on `PromptView`, `InputPromptView`, `ConfirmPromptView` and `FuzzySelectPromptView`; `FlowSummaryView`.
- `src/theme/tokens.rs`: `step_style`, `step_gap` prompt tokens; `flow_steps_word` (see below).
- `src/theme/defaults.rs`, `mod.rs`, `restyle.rs`: the counter in the default layouts; `format_flow_summary`.
- `tests/themes.rs` and snapshots: new lines only.
- `examples/flow.rs` (new; `[[example]]` with `required-features = ["password"]` if it uses Password).
- `examples/ultraviolet/flows.rs`, `theme.rs`: `setup` on `Flow`; `set_step`, `clear_step` and the `STEP` static go; the theme overrides read `view.step`.
- `docs/USAGE.md`, `DEVELOPMENT.md`, `ROADMAP.md`, PAL-002 and PAL-004 status notes.

## Public API

```rust
// paloma::flow
/// Runs the steps of one task in order over a state the app owns.
pub struct Flow<'a, S> { .. }

impl<'a, S> Flow<'a, S> {
    /// A flow over `state`; `run` returns it.
    pub fn new(state: S) -> Self
    /// A step: `f` runs app code and asks prompts, reading and writing the state.
    /// `Ok(())` moves on; `Err(Error::Back)` (Esc in a prompt) goes back;
    /// `Err` from [`back_to`] jumps back; any other error ends the flow with it.
    pub fn step(self, name: &'static str, f: impl FnMut(&mut S) -> Result<()> + 'a) -> Self
    /// A step that runs only while `condition` holds for the state.
    pub fn step_if(
        self,
        condition: impl Fn(&S) -> bool + 'a,
        name: &'static str,
        f: impl FnMut(&mut S) -> Result<()> + 'a,
    ) -> Self
    /// The flow's name, used by the summary line of `done(Done::Collapse)`.
    pub fn title(self, title: impl Into<String>) -> Self
    /// Whether prompts show the step counter. Default: on (or `Options::flows`).
    pub fn counter(self, on: bool) -> Self
    /// Whether `Esc` goes back a step (and cancels the flow on the first one).
    /// Default: on (or `Options::flows`). Off: `Esc` does nothing in `interact()`.
    pub fn esc_back(self, on: bool) -> Self
    /// What each answered prompt leaves, unless the prompt sets its own `done`.
    pub fn step_done(self, done: Done) -> Self
    /// Keeps only the output of the last `n` finished steps on screen while the flow
    /// runs. Default: all.
    pub fn keep_last(self, n: usize) -> Self
    /// What the whole flow leaves when it finishes. Default `Done::Keep`.
    /// `Collapse` without a title acts like `Erase`.
    pub fn done(self, done: Done) -> Self
    /// Draws with `theme` instead of the app theme (the summary line; prompts keep
    /// their own theme settings).
    pub fn theme(self, theme: &'a dyn Theme) -> Self
    /// Runs the steps and returns the state. `Err(Error::Cancelled)` after `^C`, or
    /// after `Esc` on the first step.
    pub fn run(self) -> Result<S>
}

/// Ends the current step and jumps back to the earlier step `step`, showing
/// `message` in the row of the first prompt that step opens.
/// `return flow::back_to("email", Message::error("Wrong email or password"));`
pub fn back_to<T>(step: &str, message: impl Into<Option<Message<'static>>>) -> Result<T>

/// Records `text` (one or more lines, already printed by the app) as output of the
/// current step, so going back and `keep_last` account for it. Does nothing outside a
/// flow.
pub fn printed(text: &str)
```

`paloma::Error` gains:

```rust
/// `Esc` in a prompt inside a flow: the step goes back. `Flow::run` handles it;
/// outside a flow it never occurs.
Back,
/// `flow::back_to(..)`: jump to an earlier step. `Flow::run` handles it.
BackTo(flow::BackTo),   // opaque: the step name and the message as plain data (Send + Sync)
```

`Display`: `"back"`, `"back to step {name}"`. `From<Error> for io::Error`: both `ErrorKind::Other`. A `back_to` naming no earlier step that runs makes `run()` return an `io::ErrorKind::InvalidInput` error naming the step.

`Options::flows(|f| ..)` with `FlowOptions { counter: bool, esc_back: bool, step_done: Done, done: Done }`. The flow's own setter wins, then `Options::flows`, then the defaults (on, on, unset, `Keep`). A prompt's `done` resolves in this order: its own value, the flow's `step_done` (own or `Options::flows`), its `Options` group, `prompts`, then `Collapse`.

Theme:

- `step: Option<(usize, usize)>` (number, total) is added to the four question views, as the last field and the last `new` argument. The default layouts draw it right after the question: `step_gap` spaces (default 2), then `format!("{n:02}/{total:02}")` in `step_style` (default `subtle`). It goes before the suffix, and before the default or choices.
- `format_flow_summary(f, ctx, &FlowSummaryView { title, steps }) -> fmt::Result`, default `defaults::format_flow_summary`. It draws like an answered line: answered mark, the title in `answered_label_style`, separator, then `"{steps} {flow_steps_word}"` in `value_style`. `flow_steps_word` is a `PromptStyle` token, default `"steps"`. It uses the `base` prompt group look (`PromptKind::Input`'s, which falls back to base).

## Implementation steps

1. **Ledger and active flow** (`src/flow.rs`):
   - `static ACTIVE: Mutex<Option<Active>>`, where `Active` holds:
     - the current step index, the counter `(n, total)` or `None` when the counter is off, `esc_back`, the resolved `step_done`;
     - `pending: Option<(MessageData)>` for `back_to`;
     - `asked: bool` for the current step;
     - `entries: Vec<(usize /*step*/, Stream, String /*text, may hold \n*/)>`.
   - Crate-private API:
     - `record(stream, text)`;
     - `current() -> Option<Snapshot>`, a copy of what prompts read;
     - `take_pending() -> Option<Message<'static>>`;
     - `mark_asked()`.

   Lock poisoning is ignored, as elsewhere. Reuse `MessageData` from `src/prompts/row.rs`: move it to a shared crate-private place if needed.
2. **Recording hook**: call `crate::flow::record(stream, &text)` right after the write at each lasting-output point, when the stream is a terminal:
   - `TermThemeRenderer::answered` (the answered line);
   - `TermThemeRenderer::keep` (the kept text);
   - `output::block::emit` (messages and blocks, with the blank lines of `space_before`);
   - the live engine's step line and finished line (`src/live/engine.rs`).

   Frames are never recorded: they are erased before a prompt ends.
3. **Rows**: a step's screen rows = the sum over its entries of `rows(text, width)` with the current terminal width (the helper in `src/theme/render.rs`; make it crate-visible). Erasing the output of steps `k..`:
   - write to stderr: move the cursor up that many rows (at most the terminal height, since rows that scrolled away cannot be reached), then `\r` and clear to the end of the screen;
   - remove those entries.

   The cursor is always at the start of a fresh line between steps.
4. **`Flow::run`**:
   - Register `Active` (error if one is set), with a guard that clears it on every exit, panics included.
   - Loop over the steps:
     - skip the steps whose condition is false;
     - compute `(n, total)` per decision 5;
     - reset `asked`, then call the step;
     - record whether it asked.
   - Outcomes:
     - `Ok` → next step. Then apply `keep_last`: if more than `n` finished steps have output, erase the flow's output, drop the oldest steps' entries from the ledger, and print the kept entries again (each to its own stream, recording them again).
     - `Err(Back)` → the nearest earlier step that asked (decision 4). Erase from that step's output onward and run it. With none, act as on the first step: cancel when `esc_back` is on. Erase the flow's output when its `done` is `Erase` or `Collapse`, and return `Err(Error::Cancelled)`.
     - `Err(BackTo)` → find the named step among the earlier steps that run (error if absent), erase from it onward, set `pending`, run it.
     - `Err(Cancelled)` → leave the screen as the prompt left it and return the error.
     - Any other error → return it; the screen stays.
   - At the end apply `done`:
     - `Keep`: nothing.
     - `Erase`: erase the flow's output.
     - `Collapse`: erase it and print the summary line with the step count, unless there is no title (then as `Erase`).
   - Return the state.
5. **Prompts**: in each prompt's `interact_on`, when a flow is active and its `esc_back` is on, run with `allow_back = true` and map `Ok(None)` to `Err(Error::Back)`. `interact_opt` is unchanged.
   - When a live frame is first drawn, call `flow::mark_asked()`. Presets and unattended answers do not mark it.
   - When the prompt opens (live), `take_pending()` puts the message into its row as `Invalid`, through the existing `Row::invalid`.
   - Pass `step` from the snapshot (or `None`) into the question views.
   - Resolve `done` with the flow's `step_done` per the cascade above.
6. **Theme**:
   - The `step` field and the counter in the default layouts.
   - The two tokens and `flow_steps_word`.
   - `FlowSummaryView` and the hook, forwarded by `Restyled`.
   - Existing snapshot lines stay byte-identical (`step: None`).
7. **Ultraviolet**:
   - `setup` becomes a `Flow` over a `Setup` state struct (`email`, `protocol`, `features`, `startup`), with five steps (`email`, `password`, `protocol`, `features`, `startup`) and their presets as today.
   - The step loop, `theme::set_step`, `theme::clear_step` and the `STEP` static go.
   - The theme's `label()` takes the view's `step` and draws the same `  01/05` as before (`step_style` = its subtle colour, gap 2).
   - The section, tasks and panel after the flow stay as they are.
   - Its golden transcripts must not change: under `--yes`, presets draw answered lines without a counter, as today.
8. **Example** `examples/flow.rs` (with `ColorfulTheme`):
   - a 4-step sign-up: email; password; a fake sign-in that returns `flow::back_to("email", Message::error("Wrong email or password"))` on the first try; a conditional "port" step (`step_if`) shown only when a "custom server" confirm is yes.
   - `.keep_last(2)` and `.done(Done::Collapse)` with a title.
9. **Docs**:
   - `docs/USAGE.md`: a "Flows" section covering the API, Esc, `back_to`, what stays on screen, the recording rule and `flow::printed`, and the counter tokens.
   - `DEVELOPMENT.md`: `flow.rs` and the rule that lasting output goes through `flow::record`.
   - ROADMAP: PAL-015 to Done when finished.
   - PAL-002 status note: Gap 6 closed.

## Tests and acceptance criteria

- **Snapshots:**
  - Built-in theme snapshots: existing lines byte-identical. New lines:
    - `prompt(Name, step 2/5)`, `input_prompt(Name, None, step 2/5)`, `confirm_prompt(Go?, None, step 1/3)`;
    - `flow_summary(Setup, 5)`.
  - Ultraviolet transcripts: byte-identical.
- **Unit tests** (`src/flow.rs`):
  - the counter numbering with `step_if` (the total grows when a condition becomes true);
  - the nearest asked step for Esc;
  - `back_to` to an unknown or later step;
  - the ledger's row count, with wrapping and both streams;
  - the `done` cascade with `step_done`.

  Drive steps without a terminal using presets and the recording sink in `src/prompts/testing.rs`.
- **Pty checks** (helper at `/tmp/claude-0/-home-user-paloma/cdf05a7f-88e2-52b1-9414-d6b777cecf3c/scratchpad/ptycmp.py`, 100×30):
  - `examples/flow.rs`:
    - the counter shows and grows from `/04` to `/05` when the conditional step joins;
    - Esc on step 2 returns to step 1 with the email prefilled, and the old answered line is gone;
    - the failed sign-in returns to the email with the error in its row;
    - with `keep_last(2)` only the last two answered steps are visible while the flow runs;
    - the collapsed summary line at the end;
    - `^C` at any step exits with the cancelled state;
    - Esc on step 1 cancels.
  - Ultraviolet `setup` (`COLORTERM=truecolor ULTRAVIOLET_MOTION=0`), with the PAL-009 key list. Screens must equal those of the previous build, except after Esc: the old answers no longer stay on screen twice (the known PAL-005/PAL-017 bug this phase fixes). Report every differing screen with its reason.
- **The full check set** from `DEVELOPMENT.md`.

## Out of scope

- Nested flows (a flow inside a step): `run` refuses.
- An option to keep stale output on back (decision 3).
- Progress bars: PAL-011.

## Status notes

- 2026-10-11: Plan written; executor Opus 5.5, medium effort. Work stops after this phase's PR until the client resumes.
- 2026-10-11: Implemented on branch `claude/dazzling-galileo-dqa12c`, uncommitted (Opus 5.5, medium). New: `src/flow.rs` (`Flow`, `back_to`, `printed`, `BackTo`, the active flow and its ledger), `examples/flow.rs`, `tests/flows.rs`. Changed: `Error` (`Back`, `BackTo`), `Options::flows` (`FlowOptions`), the seven prompts (`interact_on`, the pending message), `base_methods!` (`step_done` in the `done` cascade), the renderer (counter, asked, recording), `output::block::emit` and the live engine (recording), the four question views (`step`), `FlowSummaryView` and `format_flow_summary` (with `Restyled`), the `step_style`, `step_gap` and `flow_steps_word` tokens, the Ultraviolet `setup` command and theme, and the docs. Differences from the plan, each with its reason:
  - **The renderer does the per-prompt work it can.** `TermThemeRenderer::new` reads the counter from the flow (`flow::counter()`) and passes it to every question view, and its first frame calls `flow::mark_asked()`. So no prompt builds its views differently, and presets and unattended answers (which draw no frame) never mark the step. Only `take_pending` is in each prompt (right after `set_legend`), because the row belongs to the prompt.
  - **`interact()` in a flow runs with `allow_back`, so `q` goes back too** where `interact_opt` takes it (`Select`, `MultiSelect`, `Sort`, `Confirm`, and `FuzzySelect` in vim mode), and the legend lists `esc back`. This is the existing `Esc` path of each prompt; separating `q` would mean a second flag in every prompt. Documented in `docs/USAGE.md` and on each `interact`.
  - **`back_to` accepts the current step's own name** (the step runs again, with the message in its first prompt's row), for example to ask a password again after a failed sign-in. The plan says "an earlier step"; only unknown and later names (and steps that did not run) are errors.
  - **`Options::done` takes the flow's value as a parameter** (`done(own, flow, group)`) and `done_mode` passes `flow::step_done()`, so the cascade is unit-tested in `options.rs` without global state.
  - **Flow runs are tested in `tests/flows.rs`, not in `src/flow.rs`.** The running flow is global: a flow test in the library's test binary would change `done` (through `step_done`) for prompt tests running on other threads. `src/flow.rs` unit-tests the pure parts the plan lists (numbering with `step_if`, the nearest asked step, `back_to` targets, the ledger's rows with wrapping and both streams, `keep_last`'s cut, `interact`'s mapping, `BackTo` as `Send + Sync` data); `tests/flows.rs` (one test function, Unix) drives flows with presets on the recording `Term`: order and `step_if`, `step_done` (own, `Options::flows`, outside a flow), `back_to` earlier, current, unknown and later, `Esc` on the first step and past preset-only steps, `^C`, nesting refused, the registration cleared after an error and a panic, `Collapse` without a title.
  - **`flow::printed` records text as stderr output whatever the streams are**: the app knows it reached the screen, and `keep_last` prints it again on stderr. paloma's own output is recorded only when its term is a terminal (`flow::record_on`).
  - **A pending `back_to` message is dropped when its step ends**, so it never shows in a later step if the target step opens no live prompt.
  - **Erasing** writes to stderr, or to stdout when only stdout is a terminal, and moves up at most the terminal height. `keep_last` prints the kept entries again exactly as recorded (already styled for their stream). Entries `keep_last` dropped are not printed again when the flow goes back (in `examples/flow.rs`, `Esc` on the confirm goes back to the password and the email line, dropped earlier, stays gone).
  - **The summary line** is printed with `output::block::emit` on stderr, also when stderr is not a terminal (nothing was recorded or erased then, so a log shows every line and then the summary).
  - **`Err(Error::Back)` with `esc_back` off** (an app returning it itself) goes back like with it on.
  - `FuzzySelectPromptView` gets `step` too, and the default fuzzy layout draws it; the plan lists no snapshot line for it, so none was added.
  - With `SimpleTheme` (`gap(0)`) the suffix follows the counter directly: `Name  02/05:`.
  - Ultraviolet: `Esc` at the email step now cancels the flow (exit 130, nothing left on screen), as decision "on the first step it cancels" says; before, `Esc` did nothing there because `Input` used `interact()`. With `--email` set, `Esc` at the password also cancels (the email step was answered by its preset). Its `label()` keeps drawing `  01/05` itself, now from `view.step` with `step_style` (set to its subtle colour) and `step_gap(2)`.
  - `README.md` was not changed.
- 2026-10-11: Checks on the final tree:
  - `cargo test --locked --all-features`: 119 library unit tests, `flows` 1, `outcomes` 1, `themes` 1, `ultraviolet` 5, the Ultraviolet example's 8 unit tests, 41 doctests (1 ignored as before). Default features: 113 unit tests, the integration tests, 38 doctests. `--no-default-features`: 109 unit tests, the integration tests, 33 doctests. All pass.
  - `cargo lint`, `cargo clippy --locked --all-targets --no-default-features -- -D warnings`, `cargo format-check`, `cargo check --examples --all-features`, `RUSTDOCFLAGS=-Dwarnings cargo doc --locked --all-features --no-deps --document-private-items` and `cargo +1.85.0 check --locked --lib --all-features`: clean.
  - Known flake, not from this phase: `live::task::tests::tasks_print_steps_and_results_and_keep_one_slot` fails now and then (the preset tests of prompts pause and resume the global live region while it runs). It failed in 4 of 60 runs of the library test binary on this tree and 1 of 60 on `HEAD` (in a second batch of 80 each, 5 and 3 or 4); no other test failed.
  - Snapshots: `git diff --numstat tests/snapshots` shows 4 added lines in each of `theme-simple.txt` and `theme-colorful.txt` (`prompt(Name, step 2/5)`, `confirm_prompt(Go?, None, step 1/3)`, `input_prompt(Name, None, step 2/5)`, `flow_summary(Setup, 5)`) and nothing else; the Ultraviolet snapshots did not change.
- 2026-10-11: Pty checks (Python `pty` + `pyte`, 100×30, cells compared on character, colours, bold, italics, reverse and the visible cursor):
  - **Ultraviolet**, baseline binary built before any change (`uv-before-015` in the scratchpad) against the new one, `COLORTERM=truecolor ULTRAVIOLET_MOTION=0`, PAL-009's key list (`bad` Enter, 3 Backspaces, the email, Enter, `secret12` Enter, Down Enter, Down Space Enter, Esc, Enter, Enter, Right, Enter): `setup` and `setup --ascii` have identical screens 0 to 37; screens 38 to 43 differ, all for the intended reason: after `Esc` on the confirm the baseline kept `◇ Features · Kill switch, Split tunnelling, Auto reconnect` and drew Features again under it, the new build erases that line and draws Features in its place (same counter `04/05`), so everything after is one row higher (and the cursor one row higher at the end). `connect` (`es`, Down, Up, Enter, Enter), `favorites` (Space, Down, Space, Enter), `disconnect` (Enter), `setup --yes`, `connect --yes`, `favorites --yes`, `status`, `servers`, `about` and `connect --server xx-nowhere-01`: identical on every screen.
  - Ultraviolet, further keys: `Esc` at the password goes back to the email with `ada@example.com` prefilled and no answered line left (the baseline ignored `Esc` there); `Esc` after the password, at Protocol, goes back to the password and erases its answered line (the baseline left `◇ Password · ••••••••` above the new password prompt); `Esc` at the email exits 130 with an empty screen; `^C` at the email draws `◇ Account email · cancelled` and exits 130, as before.
  - **`examples/flow.rs`** (`ColorfulTheme`): the counter reads `01/04` to `04/04` and the port step shows `05/05` after `y` at the confirm (`Esc` there shows the confirm as `04/05`); `Esc` at the password (step 2) goes back to the email with `ada@x.io` prefilled and the old answered line gone; the failed sign-in erases the email, password and spinner lines and reopens the email with `✘ Wrong email or password` in its row; with `keep_last(2)` only `✔ Password · ********` and `✔ Signed in as ada@x.io` stay above the confirm, then only the sign-in and confirm lines above the port; at the end one line `✔ Sign up · 5 steps` (4 without the port step); `^C` at the password and at the port leaves the earlier lines and the example prints `▲ Sign-up cancelled` (it then exits with 130; the exit code was not read in these runs); `Esc` at the email cancels at once; `Esc` on the confirm goes back past the sign-in step (it asked nothing) to the password.
  - **A scratch app** (outside the repo) at 40 columns with a step that prints a 3-row info message to stdout, a section, a key/value block with a wrapping value, a task step and result, an app `println!` reported with `flow::printed`, then a select: `Esc` at the next step's confirm erases exactly that step's output and redraws it, `Esc` at the select erases all of it back to the first prompt; `keep_last(1)` drops the first answered line; the collapsed summary replaces everything.
  - **Regression**: `select`, `input`, `confirm`, `multi_select`, `sort`, `password`, `fuzzy_select`, `items` and `wizard` built from `HEAD` (in a `git worktree`, removed afterwards) against this tree: identical on every screen, except `wizard`'s last-screen cursor, which also differs between two runs of the `HEAD` binary (the known end-of-run noise).
- 2026-10-11 (planner review): accepted the deviations above. Fixed a flaky unit test the agent reported (`live::task::tests::tasks_print_steps_and_results_and_keep_one_slot`, failing about 1 in 60 runs on `main` and more often with this phase's extra prompt tests). Prompt renderers created by other tests running in parallel paused the app-wide live region during its "no frames while paused" assertions. A test-only `TEST_REGION` lock (`src/live/engine.rs`) is now held shared by every renderer and exclusively by that test. The renderer takes it before pausing the region, otherwise a waiting renderer would hold its pause and stall the test. Result: 0 failures in 80 runs (60 with default features, 20 with all features). Planner pty run of `examples/flow.rs`:
  - the counter grows from `01/04` to `05/05` when the conditional step joins;
  - the failed sign-in reopens the email with "Wrong email or password" in its row and the text kept;
  - `Esc` on the confirm skips the sign-in step and lands on the password;
  - `keep_last(2)` leaves two answers on screen;
  - the flow ends with `✔ Sign up · 5 steps`.
