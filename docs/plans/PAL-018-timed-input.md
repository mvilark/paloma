# PAL-018: Timed input: background checks and cursor style

Roadmap item: [PAL-018 in ROADMAP.md](../../ROADMAP.md). Umbrella plan: [PAL-004, Phase 5](PAL-004-ui-framework.md#pal-010--phase-5-prompt-gaps-from-pal-002), third of three parts. Needs [PAL-010](PAL-010-message-row.md) and [PAL-017](PAL-017-prompt-outcomes.md) merged first; read their Public API sections (the message row, `Row`, frames, `read_key_raw`).

## Executor

**Opus 5.5, high effort.**

| Criterion | Assessment |
| --- | --- |
| Design left open | Closed here. |
| Risk of subtle bugs | High: terminal modes (a timed read in canonical mode never wakes up), a worker thread with stale results, platform code for Windows that cannot be run here. |
| Size | Small to medium: one key-reading module, one worker loop, two tokens. |
| Verification | Pty timing checks on Linux; Windows only compiled (`cargo check --target x86_64-pc-windows-msvc` if the target is installed, otherwise a careful review). |

The Windows half can only be reviewed, not run, so the executor must get it right by reading: worth the stronger model. The planner reviews the diff and reruns every check before pushing.

## Context and decisions

Client decisions:

- 2026-10-10 (umbrella plan): "With `.change_delay(ms)` it runs in the background for slow checks (a server call): paloma keeps reading keys, waits until the user pauses for `ms`, drops results for an outdated value, and shows `.pending(Message)` while a check runs (by default the row keeps what it had). A slow `validate` shows `.pending(..)` too."
- 2026-10-10: "the live check should be disabled altogether rather than freezing the program… just declare them per platform dependencies". So `libc` on Unix and `windows-sys` on Windows are plain target-specific dependencies (no feature). They are already in the build through `console`. On any other target, background checks are disabled altogether: the closure is never called and the row keeps its description. They never fall back to running on the input thread.
- PAL-002 Gap 9: "Let a theme choose the input cursor (block, blink)." The Ultraviolet design asks for a blinking block cursor.

Decided here:

1. **A timed key read** (`read_key_timeout`) keeps the terminal in raw mode for the whole prompt while it waits. `console` only switches to raw mode inside its own read, and in canonical mode the terminal hands over input a line at a time, so a poll would not wake up on a single key.
2. **`on_change` closures are `Send`.** PAL-010 must already declare them `+ Send` (see Status notes), so this phase adds no breaking change. A background check runs the same closure on a scoped worker thread (`std::thread::scope`), so it needs no `'static` bound.
3. **Only plain data crosses threads.** `Message` holds a `&dyn Theme`, which is not `Send`. The worker turns the returned message into a crate-private `MessageData { status, text, children: Vec<MessageData> }` before sending it back.
4. **A slow `validate` needs no thread.** On submit, when `pending` is set, the prompt draws the pending row, then runs the validators on the input thread, then draws the result.
5. **Cursor style** uses the terminal's own cursor shapes (DECSCUSR, `ESC [ n SP q`). A cursor drawn by paloma that blinks on a timer is out of scope; most terminals reset the blink of their own cursor on each key anyway.

## Files

- `Cargo.toml`: `[target.'cfg(unix)'.dependencies] libc = "0.2"`; `[target.'cfg(windows)'.dependencies] windows-sys = { version = "0.61", features = ["Win32_Foundation", "Win32_System_Console", "Win32_System_Threading"] }`. Match the versions already in `Cargo.lock` (libc 0.2.189, windows-sys 0.61.2), so nothing new is downloaded.
- `src/keys.rs` (new, crate-private): `RawMode` guard and `read_key_timeout`.
- `src/prompts/row.rs`: background check state and worker; `MessageData`.
- `src/prompts/*.rs`: `change_delay`, `pending`, the timed loop; cursor shape for Input and Password.
- `src/theme/tokens.rs`: `CursorShape`, `cursor_shape` and `cursor_blink` tokens.
- `src/terminal.rs`: `restore()` also resets the cursor shape and leaves raw mode if a prompt is in it.
- `examples/prompt_messages.rs`: a `change_delay` example (a fake slow check with `thread::sleep`).
- `examples/ultraviolet/theme.rs`: block, blinking cursor.
- `docs/USAGE.md`, `DEVELOPMENT.md`, `ROADMAP.md`.

## Public API

On every prompt that has `on_change` (all of them):

```rust
/// Runs `on_change` in the background once the user has paused for `ms`
/// milliseconds, instead of after every key. Results for a value that changed in the
/// meantime are dropped. Where paloma cannot read keys with a timeout (only Unix and
/// Windows are supported), the check never runs.
pub fn change_delay(mut self, ms: u64) -> Self
/// The row while a background check or `validate` runs. Left unset, the row keeps
/// what it showed.
pub fn pending(mut self, message: impl IntoMessage) -> Self   // Status::Info
```

Tokens (`PromptStyle`, used by Input and Password):

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CursorShape { Terminal, Block, Underline, Bar }   // paloma::theme
cursor_shape: CursorShape   // default Terminal: paloma does not touch the cursor
cursor_blink: bool          // default true; ignored with Terminal
```

DECSCUSR codes: Block 1 (blinking) / 2 (steady), Underline 3 / 4, Bar 5 / 6. Write it when Input or Password starts, if the stream is a terminal. Reset with `ESC [ 0 SP q` when the prompt ends (every exit path, including errors and `^C`) and in `terminal::restore()`.

## Implementation steps

1. **`src/keys.rs`:**
   - `pub(crate) struct RawMode` (a guard).
     - On Unix it opens the same input `console` reads from: stdin if it is a terminal (`libc::isatty(0) == 1`), otherwise `/dev/tty`.
     - It saves the `termios` (`tcgetattr`) and applies `cfmakeraw` while keeping the original `c_oflag`, exactly as `console`'s `read_single_key` does, so output still translates `\n`.
     - `Drop` restores the saved `termios`. Store the saved state in a `static Mutex<Option<..>>` too, so `terminal::restore()` can restore it from a signal handler path.
     - On Windows `RawMode` is a no-op: `console` already reads console events without line buffering.
   - `pub(crate) fn read_key_timeout(term: &Term, timeout: Duration) -> io::Result<Option<Key>>`:
     - **Unix:** `libc::poll` on the input fd with the timeout in milliseconds, retrying on `EINTR` with the remaining time. If readable, `Ok(Some(term.read_key_raw()?))`, else `Ok(None)`. `console` saves and restores the current (already raw) mode around its read, so the terminal stays raw.
     - **Windows:** `GetStdHandle(STD_INPUT_HANDLE)`, then loop until the deadline:
       - `WaitForSingleObject(handle, remaining_ms)`; on `WAIT_TIMEOUT` return `Ok(None)`.
       - `PeekConsoleInputW` for one record. A `KEY_EVENT` with `bKeyDown != 0` returns `Ok(Some(term.read_key_raw()?))`.
       - Otherwise (key-up, mouse, focus or resize event) consume it with `ReadConsoleInputW` and keep waiting.
     - **Other targets:** this function does not exist. `cfg(any(unix, windows))` gates it and every caller.
   - `pub(crate) const TIMED_INPUT: bool = cfg!(any(unix, windows));`.

2. **Background checks** (`src/prompts/row.rs`):
   - `Row` gains `delay: Option<Duration>`, `pending: Option<Message<'static>>` and a background state: `generation: u64`, `due: Option<(Instant, V::Owned)>`, `running: bool`.
   - `V: ToOwned`, with `V::Owned: Send` (`str` → `String`, `[usize]` → `Vec<usize>`, `usize`, `bool`).
   - When `delay` is set and `TIMED_INPUT` is true:
     - `changed(value)` no longer calls the closure. It bumps `generation` and sets `due = (now + delay, value.to_owned())`.
     - The prompt loop wraps its body in `std::thread::scope`. It spawns one worker that receives `(generation, V::Owned)` jobs on an `mpsc` channel, locks the `on_change` mutex, calls the closure, converts the result to `Option<MessageData>`, and sends `(generation, result)` back.
     - The loop reads keys with `read_key_timeout(term, until_due_or_100ms)`.
     - When `due` has passed: send the job, set `running` (the row shows `pending` if set), clear `due`.
     - When a result arrives: drop it if its generation is not the latest; otherwise set `Change(..)` or `Description` and redraw.
     - Closing the channel ends the worker when the prompt returns.
     - Hold a `RawMode` guard for the whole prompt while `delay` is set.
   - When `delay` is set but `TIMED_INPUT` is false, `changed` does nothing (the row keeps its description).
   - Without `delay`, behaviour is exactly PAL-010's.

3. **Slow `validate`:** on submit, if `pending` is set, draw a frame with the pending row before running the validators.

4. **Cursor shape:**
   - Input and Password write the DECSCUSR sequence at start when the resolved look's `cursor_shape` is not `Terminal`.
   - A guard resets it on every exit path.
   - Ultraviolet sets `cursor_shape(Block)` with `cursor_blink(true)` in its `input` and `password` groups.

5. **`terminal::restore()`**: also write the cursor reset (only when the stream is a terminal, as for the rest of `restore`), and restore a saved raw mode.

6. **Docs**: USAGE "Message row" gains "Slow checks" (`change_delay`, `pending`, the platform note) and a "Cursor" paragraph. `DEVELOPMENT.md`: `keys.rs`, and the rule that only plain data crosses threads. ROADMAP.

## Tests and acceptance criteria

- **Snapshots:** all unchanged.
- **Unit tests:**
  - The row's generation logic: a stale result is dropped, the latest one is applied, and `pending` is shown while running. Test the state functions without threads.
  - `MessageData` round-trips nesting.
  - DECSCUSR codes for each shape and blink.
- **Pty checks** (Linux, Python `pty` + `pyte`) on `examples/prompt_messages.rs`, with a check that sleeps 500 ms and `change_delay(300)`:
  - Type 5 characters quickly (50 ms apart): no check runs while typing. The pending row appears about 300 ms after the last key; the result row appears after about 800 ms.
  - Type, pause 400 ms, then type again while the check runs: the result for the old value never shows.
  - Every key echoes immediately while a check runs (the prompt never freezes).
  - `^C` during a running check: the prompt exits cleanly, the terminal is back in cooked mode (`stty -a` from the pty child after exit shows `icanon`), and the cursor shape is reset.
- **Ultraviolet `setup`:** the cursor-shape sequence is sent at the email prompt and reset afterwards (check the raw byte stream for `\x1b[1 q` and `\x1b[0 q`).
- **Windows:** `cargo check --target x86_64-pc-windows-msvc --all-features` if the target can be added (`rustup target add`). Otherwise state in the status notes that the Windows code was reviewed but not compiled.
- **The full check set** from `DEVELOPMENT.md`, plus `cargo deny check` if available (the target-specific dependencies are already in the lockfile).

## Out of scope

- A cursor drawn by paloma with a timed blink: not planned.
- Background `validate`: not planned (decision 4).
- Flows: PAL-015.

## Status notes

- 2026-10-11: Plan written. Requirement on PAL-010: `on_change` closures must be `Send` (`impl FnMut(&V) -> Option<Message<'static>> + Send + 'a`, stored as `Arc<Mutex<dyn FnMut(&V) -> Option<Message<'static>> + Send + 'a>>`), so this phase can run them on a worker thread without another breaking change.
