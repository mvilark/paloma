# PAL-010: Prompt message row

Roadmap item: [PAL-010 in ROADMAP.md](../../ROADMAP.md). Umbrella plan: [PAL-004, Phase 5](PAL-004-ui-framework.md#pal-010--phase-5-prompt-gaps-from-pal-002), first of three parts (then [PAL-017](PAL-017-prompt-outcomes.md) and [PAL-018](PAL-018-timed-input.md)). Builds on [PAL-009](PAL-009-theme-v2-prompts.md): read its Public API section first, this plan uses its hooks, views and tokens.

## Executor

**Opus 5.5, high effort.**

| Criterion | Assessment |
| --- | --- |
| Design left open | Closed here, but the implementer must keep many prompts consistent while rewriting their loops. |
| Risk of subtle bugs | High: redrawing frames with wrapped lines, cursor placement on the last column, scrolling at the screen bottom, two-step password prompts. |
| Size | Large: every prompt, the renderer, a new line editor, tokens, one hook, docs. |
| Verification | Pty runs comparing every example with `main` screen by screen. |

A cheaper model would likely get the frame and cursor maths subtly wrong. The planner reviews the diff and reruns every check before pushing.

## Context and decisions

Today `Input` echoes typed characters itself, with hand-written cursor maths per key. An input error is drawn under the input through special bookkeeping (`pending_error`, `error_shown`), and the input is cleared after an error. List prompts redraw their items per key but keep the question line. Nothing can show a line under a prompt that changes while the user types.

Client decisions (2026-10-10, quoted from the umbrella plan):

- "Every prompt can have a row under it. The row exists only when something is put in it; there is no switch to show or hide it. Over the prompt's life it shows the latest of three things":
  1. `.description(..)`: "shown when the prompt opens, and whenever nothing newer applies. It takes `impl Into<Message>`: a plain string is shown as a description (neutral text), and any message works too."
  2. `.on_change(|value| -> Option<Message>)`: "called each time the value changes (a key typed, the selection moved). `Some(message)` replaces the row; `None` brings the description back." The value is the text for Input and Password, the highlighted index for Select and FuzzySelect, the checked items for MultiSelect, the focused choice for Confirm, the order for Sort. "paloma passes the value to the closure and never prints it (so it is safe for passwords)."
  3. `.validate(..)`: "called on submit. An error keeps the prompt open, keeps the input, and replaces the row until the value changes."
- "The row holds a `Message` of any kind, drawn with the messages tokens; the theme decides where and how the row is drawn". Nested messages (`Message::nest`) are drawn indented under it.
- "`.placeholder(\"…\")` for `Input` (and `FuzzySelect`'s search): grey text shown while the input is empty, gone at the first key, never part of the answer. Distinct from `.prefill(..)`."
- In this phase `on_change` runs on the input thread, right after each key. Background checks (`change_delay`, `pending`) are [PAL-018](PAL-018-timed-input.md).
- From PAL-002: in the Ultraviolet design the input caret `›` turns `error` while the row shows a validation error (Gap 4).

Decided here:

1. **Every prompt draws whole frames.** On every key the prompt rebuilds its full frame (question line, items, typed text, row) from its state, and the renderer replaces the previous frame. This replaces Input's per-key echo and cursor maths and the error bookkeeping.
2. **The typed text is drawn by paloma, after the input hook.** `format_input_prompt` keeps its contract (it draws up to where the text goes). The renderer then writes the text in the prompt's `text_style`, or the mask glyphs, or the placeholder in `placeholder_style`, and places the terminal cursor.
3. **`IntoMessage`**, a public trait, turns a value into a `Message` with a status chosen by the caller. It is implemented for `Message<'static>` (keeping its own status) and for every `Display` type (using the caller's status). So `.description("text")` is a description, a validator's `Err("text")` is an error, and validators returning any `Display` error keep compiling. `Message` must never implement `Display`, or the two impls would overlap; say so in a comment on `Message`.
4. **The row is drawn below everything the prompt draws** (after the items of a list prompt), one line per message and nested message, by a new hook `format_prompt_message`.
5. **Validation for list prompts too.** Select, MultiSelect, Sort and FuzzySelect get `.validate(..)` (for example "pick at least one"). Confirm gets none.
6. **Error state.** While the row shows a validation error, the question views carry `invalid: true`. The default layout then draws the suffix in `invalid_suffix_style`, which defaults to `suffix_style`, so the built-in themes look the same.

## Files

- `src/output/message.rs`: `IntoMessage` trait; the comment on `Message`.
- `src/lib.rs`: re-export `IntoMessage`.
- `src/prompts/line.rs` (new): `LineEditor`, the text and cursor of a one-line input.
- `src/prompts/row.rs` (new): `Row<'a, V>`, the description, `on_change` closure and row state shared by all prompts.
- `src/prompts/base.rs`: `PromptBase` gains `description`; `base_methods!` gains `.description(..)`.
- `src/prompts/*.rs`: every prompt draws frames and gets `on_change`; the list prompts get `validate`; Input and FuzzySelect get `placeholder`.
- `src/theme/render.rs`: frames replace heights; the error bookkeeping goes.
- `src/theme/prompt.rs`: `invalid` on the question views; `PromptMessageView`, `MessageSource`.
- `src/theme/tokens.rs`: `message_indent`, `invalid_suffix_style`, `placeholder_style`; `text_style` documented as the typed text of every prompt.
- `src/theme/defaults.rs`, `mod.rs`, `restyle.rs`: `format_prompt_message`; `invalid` handling; placeholder in the fuzzy search line.
- `src/validate.rs`: validator errors bound by `IntoMessage` instead of `ToString`.
- `src/paging.rs`: capacity leaves room for the row.
- `tests/themes.rs` and snapshots: new lines for the new hook and the `invalid` state.
- `examples/prompt_messages.rs` (new) and its `[[example]]` entry if a feature is needed (it uses `password`).
- `examples/ultraviolet/theme.rs`: tokens only (see step 9).
- `docs/USAGE.md`, `DEVELOPMENT.md`, `ROADMAP.md`.

## Public API

```rust
// paloma (src/output/message.rs)
/// A value that can be shown as a message: a `Message` keeps its own status, any
/// other `Display` value becomes a message of the status the caller picks.
pub trait IntoMessage {
    /// The message; `status` applies when `self` is plain text.
    fn into_message(self, status: Status) -> Message<'static>;
}
impl IntoMessage for Message<'static> { /* returns self */ }
impl<T: std::fmt::Display> IntoMessage for T { /* Message::new(status, self.to_string()) */ }
```

On every prompt (through `base_methods!`):

```rust
/// A line under the prompt, shown while nothing newer applies (see `on_change` and
/// `validate`). Plain text is a description; any `Message` works too.
pub fn description(mut self, message: impl IntoMessage) -> Self   // Status::Description
```

Per prompt (`V` is what the closure receives):

| Prompt | `on_change(f: impl FnMut(&V) -> Option<Message<'static>> + Send + 'a)` | `validate` |
| --- | --- | --- |
| `Input<T>` | `V = str` (the raw text, before parsing) | unchanged signature, `V::Err: IntoMessage` |
| `Password` | `V = str` | unchanged signature, `V::Err: IntoMessage` |
| `Confirm` | `V = bool` (the focused or chosen answer; called only once one exists) | none |
| `Select` | `V = usize` (the highlighted item; only once one is highlighted) | `validate(f: impl FnMut(&usize) -> Result<(), E> + 'a) where E: IntoMessage` |
| `FuzzySelect` | `V = usize` (index into the items, not into the filtered list) | as Select |
| `MultiSelect` | `V = [usize]` (indices of the checked items, ascending) | `validate(f: impl FnMut(&[usize]) -> Result<(), E> + 'a)` |
| `Sort` | `V = [usize]` (the current order) | as MultiSelect |

Several `validate` calls chain (all run in order, the first error wins), as `Input::validate` does today. Several `on_change` calls: the last one wins. `on_change` closures are `Send` so that [PAL-018](PAL-018-timed-input.md) can run them on a worker thread without another breaking change; `validate` closures need not be.

`Input::placeholder(text: impl Into<String>)` and `FuzzySelect::placeholder(text: impl Into<String>)`: shown while the text is empty.

Theme:

```rust
// src/theme/prompt.rs
pub enum MessageSource { Description, Change, Validation }        // non_exhaustive, Copy
pub struct PromptMessageView<'a> {                                 // non_exhaustive
    pub kind: PromptKind,
    pub status: Status,
    pub text: &'a str,
    pub depth: usize,             // 0 for the message, 1+ for nested messages
    pub source: MessageSource,
}
impl PromptMessageView { pub fn new(kind, status, text, depth, source) -> Self }
// `invalid: bool` is appended to PromptView, InputPromptView, ConfirmPromptView and
// FuzzySelectPromptView (and to their `new`, last argument).
// FuzzySelectPromptView also gains `placeholder: Option<&'a str>` (after `cursor`).

// Theme
/// Formats one line of the message row under a prompt. The default is
/// [`defaults::format_prompt_message`]: `message_indent` spaces, then the line as
/// `format_message` draws it.
fn format_prompt_message(&self, f, ctx: &Context, view: &PromptMessageView) -> fmt::Result
```

New `PromptStyle` tokens (with `PromptLook` fields): `message_indent: usize` (default 0), `invalid_suffix_style: Style` (default: `suffix_style`), `placeholder_style: Style` (default: `subtle`). `text_style` doc becomes "The typed text: input, password mask, search. Default: `text`."

Breaking: `validate` error types need `IntoMessage` (every `Display` type has it); the question views' `new` take `invalid`; `FuzzySelectPromptView::new` takes `placeholder`.

## Implementation steps

1. **`IntoMessage`** in `src/output/message.rs` with the two impls; re-export from `lib.rs`. Unit test: `"x".into_message(Status::Error)` has status Error; `Message::hint("x").into_message(Status::Error)` keeps Hint.

2. **`LineEditor`** (`src/prompts/line.rs`), crate-private: `chars: Vec<char>`, `pos: usize` (in chars). Methods:
   - `new(initial: &str)`: cursor at the end.
   - `text() -> String`; `before_cursor() -> String`; `is_empty()`; `set(text)` (cursor at the end).
   - `insert(c)`, `backspace()`, `delete()`, `left()`, `right()`, `home()`, `end()`, `word_left()`, `word_right()`. Each returns whether the text changed.
   - Word moves behave like today's `Alt+B` / `Alt+F` in `src/prompts/input.rs`: skip whitespace, then move to the start of the word (left) or to the next whitespace after skipping it (right).
   - `handle(key: &Key) -> Edit`, where `enum Edit { Changed, Moved, None }`, maps `Char` (not control), `Backspace`, `Del`, `ArrowLeft`, `ArrowRight`, `Home`, `End`, `UnknownEscSeq(['b'])`, `UnknownEscSeq(['f'])`.

   Unit tests for each method, including multi-byte characters.

3. **Frames in the renderer** (`src/theme/render.rs`). Replace `height`, `prompt_height`, `prompts_reset_height`, `pending_error`, `error_shown` and the `write_formatted_*` helpers with:
   - State: `kept: usize` (rows of committed frames above the live one, still part of the prompt), `rows: usize` (rows of the live frame), `cursor_row: usize` (row of the live frame the terminal cursor is on).
   - `fn frame(&mut self, text: &str, cursor: Option<(usize, usize)>) -> Result`: `text` is the whole frame without a final newline; `cursor` is `(line index, column)` in that text, or `None` to leave the cursor after the last line.
     1. Move up `cursor_row` rows, write `\r`, then `clear_to_end_of_screen`.
     2. Write `text` in one `write_str`.
     3. Count `rows` with the existing `rows(text, width)` helper. Turn `(line, column)` into a screen row and column, counting wrapped rows of earlier lines: row = rows of lines before + column / width, column = column % width.
     4. If that row is past the last row (the cursor sits right after a line that exactly fills the width), write `\r\n` and count one more row.
     5. Move up from the last row to the target row, write `\r`, move right to the column. Set `cursor_row`.
   - `fn commit(&mut self)`: the live frame stays on screen as part of the prompt (Password's first entry before its confirmation). Move the cursor to the frame's last row, write `\r\n`, then `kept += rows`, `rows = 0`, `cursor_row = 0`.
   - `fn erase(&mut self)`: move up `cursor_row + kept`, `\r`, `clear_to_end_of_screen`, reset all three to 0.
   - `answered(..)` is drawn after `erase()` as today, with `write_line`.
   - Build each frame's text with the hooks, joined by `\n`.
     - `prompt_frame(kind, prompt, paging, invalid, items: impl Iterator<Item = ItemView>, row)`: list prompts.
     - `input_frame(kind, prompt, default, invalid, shown: Shown, row)`: Input and Password. `enum Shown<'s> { Text { before: &'s str, after: &'s str }, Placeholder(&'s str), Mask { before: usize, after: usize, glyph: char }, Hidden }`.
     - `confirm_frame(prompt, default, invalid, buttons: Option<bool>, row)`.
     - `fuzzy_frame(prompt, search, cursor, placeholder, invalid, items, row)`.
   - The cursor column for input is the last line's display width after the input hook, plus the width of the text (or mask) before the cursor. With a placeholder, the cursor sits before it.
   - Keep `Pause` and the theme handle as they are. Input and Password show the terminal cursor; the other prompts keep it hidden as today.

4. **`Row`** (`src/prompts/row.rs`): `pub(crate) struct Row<'a, V: ?Sized>`, holding `on_change: Option<Arc<Mutex<dyn FnMut(&V) -> Option<Message<'static>> + Send + 'a>>>` and `state: RowState`, where `enum RowState { Description, Change(Message<'static>), Invalid(Message<'static>) }`. It derives `Clone`, so prompts stay `Clone`.
   - `changed(&mut self, value: &V)`: call `on_change` and set `Change(m)` or `Description`; without `on_change`, set `Description`.
   - `invalid(m)`: set `Invalid(m)`.
   - `current<'d>(&'d self, description: Option<&'d Message<'static>>) -> Option<(&'d Message<'static>, MessageSource)>`.
   - `is_invalid()`.

   The renderer flattens a `(message, source)` into `PromptMessageView` lines, the message at depth 0 and nested ones deeper, following `Message::render` in `src/output/message.rs`. Unit tests for the state machine: description first, change replaces it, `None` brings it back, invalid replaces until the next change.

5. **Theme parts**: the views' new fields, `MessageSource`, `PromptMessageView`, the hook with its default in `defaults.rs` (indent, then `format_message(f, ctx, &MessageView::new(status, text, depth))`), forwarding in `restyle.rs`, and the three tokens.
   - Default layouts: when `invalid`, paint the suffix with `invalid_suffix_style`.
   - `format_fuzzy_select_prompt`: with an empty search and a placeholder, draw the cursor, then the placeholder in `placeholder_style`.

6. **Prompts**, one at a time:
   - **Input:**
     - Keep `prefill`, `default`, `show_default`, `allow_empty`, `answered_prompt`, history (`↑` / `↓` set the editor's text, same positions as today) and completion (`→` at the end of the text and `Tab` replace the text).
     - After every key: if the text changed, call `row.changed(text)`; then draw a frame.
     - On `Enter`, apply the same rules as today (default when empty, `allow_empty`, parse, validators). A parse or validation error calls `row.invalid(..)` (parse errors become `Status::Error` messages) and redraws with the text kept. A success erases the frame and reports as today.
   - **Password:**
     - Read key by key (`term.read_key()`) for both masked and hidden input; `read_secure_line` goes. The mask comes from `password_mask_glyph` as today; without a mask, the text is `Shown::Hidden`.
     - After the first entry with a confirmation prompt, `commit()` and run the second entry in a new frame.
     - On a mismatch or a failed validation, `erase()`, set the first entry's row to `Invalid(..)` and start again, as today.
   - **Confirm:** both layouts draw `confirm_frame`. `on_change` gets the new value when `y` / `n` / arrows change it.
   - **Select, MultiSelect, Sort:**
     - Draw `prompt_frame` on every key; `clear_preserve_prompt` and `Paging::render_prompt` go.
     - The paging counter is passed whenever paging is active.
     - On `Enter`, run the validators; an error sets `Invalid` and redraws.
   - **FuzzySelect:** draw `fuzzy_frame`. `on_change` gets the original item index whenever the highlighted item changes. The placeholder shows while the search is empty.

7. **Paging** (`src/paging.rs`): add `reserved: usize` (rows the row takes) with `set_reserved(rows)`, called before each frame with the row's line count. Capacity becomes `max_capacity.clamp(3, term_rows) - 2 - reserved`, at least 1. Keep the resize handling.

8. **Validators** (`src/validate.rs`, Input, Password): bound `V::Err: IntoMessage` instead of `ToString`, and convert with `Status::Error`. Update every doc example that relied on `ToString`.

9. **Ultraviolet theme** (`examples/ultraviolet/theme.rs`), tokens only:
   - `message_indent(2)` (its row sits at `col-2`).
   - `invalid_suffix_style` set to its `error` colour (its caret turns `error`, Gap 4).
   - `text_style(Style::new())` in the `input` and `password` groups. Typed text was unstyled before; keep it so its look does not change.
   - Its `format_input_prompt` override draws the caret: use `invalid_suffix_style` when `view.invalid`.

10. **Example** `examples/prompt_messages.rs`:
    - An `Input` with `.description("3 to 16 letters or digits")`, a `.placeholder("ada")` and a validator.
    - A `Password` with the `on_change` rule list from the umbrella plan (nested hints under `Message::error("Password too weak")`).
    - A `MultiSelect` with `validate` requiring at least one item.

    Draw it with `ColorfulTheme`.

11. **Docs**: a "Message row" section in `docs/USAGE.md` (the three sources and their precedence, `IntoMessage`, `placeholder`, the hook and tokens); `DEVELOPMENT.md` layout lines for `line.rs` and `row.rs` and the frame model in `render.rs`; ROADMAP.

## Tests and acceptance criteria

- **Golden snapshots:**
  - `tests/snapshots/theme-*.txt`: existing lines byte-identical.
  - New lines in `tests/themes.rs` (sheet labels as given):
    - `prompt_message(Description, Text, 0)`, `prompt_message(Error, Text, 1)`;
    - `input_prompt(Name, None, invalid)`, `confirm_prompt(Go?, None, invalid)`, `select_prompt(Pick, invalid)`;
    - `fuzzy_select_prompt(Search, , 0, placeholder)`.
  - `tests/snapshots/ultraviolet-*.txt`: unchanged (no interactive output in them).
- **Unit tests:** `LineEditor`, `Row`, `IntoMessage`, the frame cursor maths (factor it as a pure function `cursor_position(lines: &[&str], line: usize, column: usize, width: usize) -> (row, column, extra_row: bool)` and test wrapping and the exact-width case).
- **Pty checks** (Python `pty` + `pyte`, 100×30 unless said otherwise), comparing `main` and this branch screen by screen where noted. The script from PAL-009 compares characters, colours, attributes and the visible cursor after each key; send each key, wait 0.35 s, snapshot.
  - Must match `main` screen by screen:
    - `select`: down, down, Enter, down, Enter, down, Enter.
    - `paging` (rows 8): down ×6, right, right, Enter.
    - `multi_select`: down, space, down, space, Enter, space, Enter, Enter.
    - `sort`: space, down, down, space, Enter, space, down, space, Enter.
    - `fuzzy_select`: `a`, `n`, left, down, Enter.
    - `confirm`: `y`, `n`, Enter, `y`, `n`, Esc, Enter, Enter, Enter.
    - `wizard`: `y`, Enter, `host`, Enter, down, Enter, Enter, Enter.
    - Ultraviolet `connect` and `favorites` (`COLORTERM=truecolor ULTRAVIOLET_MOTION=0`).
  - **Input** (`examples/input.rs`): typing, `←` and `→`, `Alt+B` and `Alt+F` (`\x1bb`, `\x1bf`), Backspace in the middle of the text. Every screen must equal `main` except after a failed validation: there the text stays and the error is on the row.
  - **Long input:** type 150 characters in an 80-column terminal, then `Home` and `End`. The cursor follows, and no stray rows are left after Enter.
  - **Last line:** run `select` with the prompt at the bottom of a full screen (print 40 lines first). Frames scroll correctly and nothing is left after Enter.
  - **Password** (`examples/password.rs`): `ab` Enter shows the validation error on the row with the masked text kept; then the full entry, confirmation, a mismatch, and success. The final screen equals `main`'s.
  - **`examples/prompt_messages.rs`:** the description shows at open, the rules disappear one by one while typing `Abcdefg1`, and the error row appears and disappears as documented.
  - **Ultraviolet `setup`:** the same keys as in PAL-009 (`bad` Enter first). The caret turns `error` while the error row shows, and the row is indented 2. Other screens equal `main`, except that the email text stays after the error.
- **The full check set** from `DEVELOPMENT.md`, including tests with all, default and no features, `cargo lint`, `cargo doc` with warnings denied, and MSRV 1.85.

## Out of scope

- `done`, Esc back for Input and Password, `^C` handling, presets, unattended mode, rich items, key legends: [PAL-017](PAL-017-prompt-outcomes.md).
- `change_delay`, `pending`, the timed key read, cursor style: [PAL-018](PAL-018-timed-input.md).
- Step counters and `back_to` with a message: PAL-015.
- Moving the Ultraviolet example onto the row (descriptions on its prompts): the Ultraviolet update after all phases.

## Status notes

- 2026-10-11: Plan written (PAL-010 split into PAL-010, PAL-017 and PAL-018 at the client's request: "unfold it in two or three phases, also that way we can fine tune the right agent for each job").
