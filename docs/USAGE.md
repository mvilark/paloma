# Using the library

This crate, `paloma`, is a fork of dialoguer. Every component is a builder: configure a prompt, then call `interact()`; configure a message, then call `print()`. All of them return `paloma::Result<T>`; its `Error` is `IO`, `Cancelled` (the user pressed Ctrl+C) or `NoDefault` (an unattended prompt had nothing to answer with), and is `#[non_exhaustive]`. `Esc` and `Ctrl+C` are described under [Back and cancel](#back-and-cancel).

```toml
[dependencies]
paloma = "0.12"   # default features: editor, password
# optional features: fuzzy-select, history, completion
```

```rust
use paloma::{theme::ColorfulTheme, Confirm, Input, Select};

fn main() -> paloma::Result<()> {
    paloma::set_theme(ColorfulTheme); // once, for every prompt
    let name: String = Input::new().prompt("Name").interact()?;
    let env = Select::new()
        .prompt("Environment")
        .items(["dev", "staging", "prod"])
        .default(0)
        .interact()?;
    if Confirm::new().prompt(format!("Deploy {name} to #{env}?")).interact()? {
        println!("deploying");
    }
    Ok(())
}
```

Every prompt has `new()` (uses the app theme from `set_theme`, `SimpleTheme` by default), `theme(&dyn Theme)` (a theme for this prompt only), `prompt(..)`, `done(Done)` (see [What an answered prompt leaves](#what-an-answered-prompt-leaves)), `legend(bool)` (see [Key legends](#key-legends)), `preset(..)` (see [Presets and unattended runs](#presets-and-unattended-runs)), `description(..)`, `on_change(..)` (see [Message row](#message-row)), and `interact_on(&Term)` to target a specific terminal. Every prompt also has `interact_opt()` / `interact_on_opt()`, which return `None` when the user goes back with `Esc` (see [Back and cancel](#back-and-cancel)).

## Confirm
```rust
let ok: bool = Confirm::new().prompt("Continue?").default(true).interact()?;
let maybe: Option<bool> = Confirm::new().prompt("Continue?").interact_opt()?;
```

`.choices_as_buttons(true)` shows `Yes` / `No` buttons on the line under the question instead of the classic `(y/n)` text: `←`/`→`/`Tab` move the focus, `Enter` picks it, and `y` / `n` jump to a button. The theme draws the buttons in `format_confirm_buttons` (the `button_indent` token indents them, 0 by default). `.on_shortcut(false)` makes `y` / `n` only select, so the user hits `Enter` to submit; it works with and without buttons. What the answered prompt leaves is set with `done`. `Esc` and `q` go back in `interact_opt`.
Other options: `show_default`.

## Input
Generic over any `T: Clone + FromStr + ToString`; use `interact()`.
```rust
let port: u16 = Input::new().prompt("Port").default(8080).interact()?;
let email: String = Input::new()
    .prompt("Email")
    .validate(|s: &String| -> Result<(), &str> {
        if s.contains('@') { Ok(()) } else { Err("needs an @") }
    })
    .interact()?;
```
Other options: `prefill` (editable text the input opens with), `placeholder` (grey text while the input is empty, never the answer), `allow_empty`, `show_default`, `answered_prompt`, `history(&mut H)` (feature `history`), `completion(&C)` (feature `completion`). Keys: `←` / `→`, `Home` / `End`, `Alt+B` / `Alt+F` by word, `Backspace` / `Del`. A refused value (parse error or validator) keeps the text and shows the error in the row under the input.

## Select
Returns the chosen index.
```rust
let i = Select::new().prompt("Pick").items(["a", "b", "c"]).default(1).interact()?;
```
Other options: `item`, `max_length` (paging), `validate` (with the chosen index). Items can be plain values or [`Item`s](#items). `interact_opt` returns `None` on Esc/`q`.

## MultiSelect
Returns the selected indices.
```rust
let picked: Vec<usize> = MultiSelect::new()
    .prompt("Features")
    .items_checked([("fast", true), ("safe", false)])
    .validate(|checked: &[usize]| if checked.is_empty() { Err("Pick at least one") } else { Ok(()) })
    .interact()?;
```

## Sort
Returns the items' indices in the user's new order.
```rust
let order: Vec<usize> = Sort::new().prompt("Rank").items(["x", "y", "z"]).interact()?;
```
`validate` gets the order (item indices, first to last).

## Password (feature `password`, on by default)
```rust
let pw = Password::new()
    .prompt("Password")
    .confirmation("Repeat", "Mismatch")
    .interact()?;
```
Other options: `allow_empty_password`, `mask_char(char)`, `validate` (takes `Fn(&String) -> Result<(), E>`; a refused password stays as typed with the error in the row). paloma reads the password key by key (`←` / `→`, `Backspace` and `Del` edit it); with a confirmation, the first entry stays on screen above the second, and a mismatch starts over with the error under the first entry.

While typing, `.mask(true)` echoes the theme's `mask` glyph for each key press (`.mask_char('•')` picks the character yourself). After the password is confirmed, the one-line result shows a fixed number of mask characters, 8 by default, set with `.result_len(n)`; themes draw it in `format_answered` (as `Answer::Hidden(len)`; the `hidden_value` token shows a fixed text instead).

## FuzzySelect (feature `fuzzy-select`)
Type to filter; returns the index into the original items.
```rust
let i = FuzzySelect::new().prompt("Search").items(["apple", "banana"]).interact()?;
```
Other options: `prefill`, `placeholder` (shown while the search is empty), `highlight_matches`, `vim_mode` (with it, `Esc` enters vim mode and `q` goes back), `max_length`, `validate` (with the chosen index into the original items). Items can be plain values or [`Item`s](#items); the search matches their text.

## Editor (feature `editor`, on by default)
Opens `$EDITOR`; returns `None` if not saved when `require_save(true)`.
```rust
let text: Option<String> = Editor::new().extension(".md").edit("initial text")?;
```

## History and completion
`BasicHistory::new().max_entries(8).no_duplicates(true)` implements `History`; pass `&mut` via `Input::history`. Implement `Completion::get(&self, input: &str) -> Option<String>` for tab-completion via `Input::completion`. See `examples/history.rs`, `history_custom.rs`, `completion.rs`.

## Message row
Every prompt can have a row under it (below its items, for list prompts). The row exists only when something is put in it, and it shows the latest of three things:

1. `.description(..)`: shown when the prompt opens and whenever nothing newer applies. A plain string is a description (neutral text); any `Message` works too (`Message::hint(..)`, `Message::warning(..)`).
2. `.on_change(|value| -> Option<Message>)`: called each time the value changes, right after the key. `Some(message)` replaces the row; `None` brings the description back. The value is the raw text for `Input` and `Password` (`&str`), the highlighted index for `Select` and `FuzzySelect` (into the original items), the checked indices for `MultiSelect` (ascending), the order for `Sort` (`&[usize]`) and the focused or chosen answer for `Confirm` (`&bool`). paloma passes the value to the closure and never prints it, so it is safe for passwords. A later `on_change` replaces an earlier one; the closure must be `Send`.
3. `.validate(..)`: called on submit (`Input`, `Password`, `Select`, `MultiSelect`, `Sort`, `FuzzySelect`). An error keeps the prompt open, keeps the input, and replaces the row until the value changes. Several validators run in order and the first error wins. While the row shows a validation error, the question line is drawn in its error state (`invalid` on the views).

```rust
use paloma::{Message, Password};

let password = Password::new()
    .prompt("Password")
    .description("Choose a strong password")
    .on_change(|pw: &str| {
        let missing: Vec<&str> = [
            (pw.chars().count() < 8, "At least 8 characters"),
            (!pw.chars().any(|c| c.is_ascii_digit()), "One digit"),
        ]
        .into_iter()
        .filter_map(|(fails, rule)| fails.then_some(rule))
        .collect();
        (!missing.is_empty()).then(|| {
            missing.into_iter().fold(Message::error("Password too weak"), |msg, rule| {
                msg.nest(Message::hint(rule))
            })
        })
    })
    .interact()?;
```

A message's nested messages are drawn indented under it, one line each. Descriptions and validation errors take any `IntoMessage`: a `Message` keeps its own status, and any other `Display` value becomes a description (for `description`) or an error (for `validate`), so validators that return `&str`, `String` or an error type keep working, and a validator can return `Err(Message::warning(..).nest(Message::hint(..)))`. A parse error of `Input<T>` shows as an error. `.placeholder("…")` (`Input`, and `FuzzySelect`'s search) is grey text shown while the input is empty; it disappears at the first key and is never the answer, unlike `.prefill(..)`. See `examples/prompt_messages.rs`.

The theme draws each line of the row with `format_prompt_message` (a `PromptMessageView`: the prompt's kind, the message's status, text and depth, and its `MessageSource`: `Description`, `Change` or `Validation`). The default writes the prompt's `message_indent` token as spaces, then the line as `format_message` draws it, with the message tokens. Other prompt tokens: `invalid_suffix_style` (the suffix while the row shows an error; defaults to `suffix_style`), `placeholder_style` (defaults to the `subtle` role) and `text_style` (the typed text: input, password mask, search). A list prompt leaves room for the row when it pages its items.

**Slow checks.** By default `on_change` runs right after each key, on the input thread, which suits fast checks. For a slow one (a server call), `.change_delay(ms)` runs it in the background once the user has paused for `ms` milliseconds: paloma keeps reading keys meanwhile, a result for a value that changed in the meantime is dropped, and the row keeps what it showed until the latest result arrives. `.pending(..)` sets what the row shows while a check runs (plain text is an info message); it also shows on submit while the validators run. A prompt that ends while a check runs returns once that check has finished. This needs a key read with a timeout, which paloma has on Unix and Windows; on any other target the check never runs (the row keeps its description), it never falls back to blocking the input. The closure gets its own copy of the value on the worker thread; for `Input` and `Password` the copy is wiped when dropped.

```rust
Input::<String>::new()
    .prompt("Username")
    .change_delay(300)
    .pending("Checking the name…")
    .on_change(|name: &str| server_says_taken(name).then(|| Message::warning(format!("{name} is taken"))))
    .interact()?;
```

**Cursor.** The `cursor_shape` prompt token (`CursorShape::Terminal`, the default, leaves the terminal's cursor alone; `Block`, `Underline`, `Bar`) with `cursor_blink` (default true) sets the terminal cursor's shape while an `Input` or `Password` is open, and gives the terminal its own shape back when the prompt ends, however it ends. `paloma::terminal::restore()` resets it too.

## What an answered prompt leaves
`.done(Done)` on any prompt (and `done` in `Options`) says what stays on the screen once the user answers:
```rust
use paloma::{Done, Select};

let i = Select::new().prompt("Pick").items(["a", "b"]).done(Done::Keep).interact()?;
```
* `Done::Collapse` (the default): one line with the answer, drawn by the theme's `format_answered`. A list prompt without a question (`Select`, `MultiSelect` and `Sort` accept none) leaves nothing, as `Erase`.
* `Done::Keep`: the prompt stays as it was when answered, without its [message row](#message-row) and [legend](#key-legends).
* `Done::Erase`: nothing stays.

It is resolved as the prompt's own value, then its group in `Options` (`select`, `input`, …), then `prompts`, then `Collapse`: `Options::new().prompts(|p| p.done(Done::Erase)).input(|i| i.done(Done::Collapse))`. `done` replaces the old `report(bool)` (`report(false)` is `done(Done::Erase)`) and the `clear(bool)` of `Select`, `MultiSelect`, `Sort` and `FuzzySelect` (`clear(false)` is `done(Done::Keep)`, which no longer also prints the answered line). `GeneralOptions::report` is gone: use `prompts().done`.

## Back and cancel
* **`Esc` goes back.** Inside a [flow](#flows), `interact()` returns `Err(Error::Back)` on `Esc` and the flow goes back a step. Everywhere, `interact_opt()` / `interact_on_opt()` exist on every prompt, `Input` and `Password` included, and return `Ok(None)` on `Esc` (and `q` where it works today: `Select`, `MultiSelect`, `Sort`, `Confirm`, and `FuzzySelect` in vim mode, where `Esc` enters vim mode). The prompt is **erased**, whatever `done` says, so a flow that goes back leaves nothing behind (`Confirm` no longer leaves a "skipped" line). `interact` ignores `Esc` outside a flow.
* **`Ctrl+C` cancels.** paloma installs no signal handler: prompts read `^C` as a key and erase themselves. By default paloma then restores the terminal and **exits with code 130** (the shell convention), so an app needs no code for it. The same happens on `Esc` in a [flow](#flows) with no step to go back to. Two app-wide options change it:
```rust
paloma::set_options(paloma::Options::new().general(|g| g
    .cancel_message(paloma::Message::warning("Cancelled"))   // printed on its own line first
    .cancel_exits(false)));                                   // return Err(Error::Cancelled) instead
```
`cancel_message` (plain text is a description, any `Message` works) is printed on a line of its own where the prompt was, never folded into the prompt's answer line; by default nothing is printed. With `cancel_exits(false)`, prompts (from `interact` and `interact_opt`) and `Flow::run` return `Err(Error::Cancelled)` and the app decides what to print and how to exit. `Error::Cancelled` converts to an `io::Error` of kind `Interrupted`. A spinner (`Task`) or a progress bar (`Progress`) is not reading keys, so it still needs your own handler (see [Tasks](#tasks)).

## Presets and unattended runs
`.preset(Option<T>)` skips a prompt and answers with a value you already have, such as a command-line flag. `Some(value)` returns it without needing a terminal and draws the line an answered prompt leaves, as `done` configures (`Keep` draws like `Collapse`, since nothing was shown; `Erase` draws nothing). A preset is used as given: it is not validated and `on_change` is not called. `None` asks as usual.

| Prompt | `T` | Unattended default |
| --- | --- | --- |
| `Confirm` | `bool` | `.default(..)` |
| `Input<T>` | `T` | `.default(..)` |
| `Password` | `String` | none (always `NoDefault`) |
| `Select` | `usize` | `.default(..)` if set |
| `MultiSelect` | `Vec<usize>` | the items checked by default (may be none) |
| `Sort` | `Vec<usize>` | the original order |
| `FuzzySelect` | `usize` | `.default(..)` if set |

With `Options::new().general(|g| g.unattended(true))` prompts without a preset return their default the same way (for a default on a disabled item, the next enabled item), so a program can run in CI or with `--yes` without a terminal. A prompt with no preset and no default returns `Err(Error::NoDefault)`. Without a preset or unattended mode, a prompt on something that is not a terminal still returns an I/O error. A preset or default that names an item that does not exist is an I/O error too.
```rust
let name: String = Input::new().prompt("Name").preset(args.name.clone()).interact()?;
```

## Items
`Select`, `MultiSelect` and `FuzzySelect` (not `Sort`, where disabling an item makes no sense) take plain values or [`Item`]s: `item(..)` and `items(..)` accept anything that is `IntoItem`, which is `Item` and every `Display` type, so `.items(&["a", "b"])` keeps working.
```rust
use paloma::{Item, Select};

let i = Select::new()
    .prompt("Plan")
    .item(Item::new("Free").description("1 device"))
    .item(Item::new("Enterprise").description("talk to sales").disabled(true))
    .interact()?;
```
`description(..)` is drawn after the item (`item_description_gap` spaces, in `item_description_style`). A **disabled** item is drawn in `disabled_style` (muted and struck through by default) and cannot be focused, chosen or checked: arrows, `Tab`, `j` / `k` and page moves skip it (wrapping around), the initial highlight and `Select`'s `default` move to the next enabled item, `Space` and `a` (check all) in `MultiSelect` leave it alone, and `Enter` in `Select` and `FuzzySelect` does nothing when no item is enabled (in `MultiSelect` it still submits what is checked). A disabled item keeps its checked state in `MultiSelect`: one checked by default stays checked and is returned. `FuzzySelect` matches on the text only. The answered line shows the text. See `examples/items.rs`.

## Key legends
`.legend(true)` (or `legend` in `Options`) puts a line with the prompt's keys at the bottom of its frame, below the message row, drawn by the theme's `format_legend`. It is off by default. The words are tokens, so a theme can translate them: `legend` (a list of `(key, action)`), `legend_back` (`("esc", "back")`, added while `Esc` goes back, in `interact_opt`), `legend_separator`, `legend_key_style` and `legend_style`. The defaults:

| Prompt | Legend |
| --- | --- |
| `Input`, `Password` | `enter submit` |
| `Select`, `FuzzySelect` | `↑↓ move`, `enter choose` |
| `MultiSelect` | `↑↓ move`, `space check`, `a all`, `enter submit` |
| `Sort` | `↑↓ move`, `space pick up`, `enter submit` |
| `Confirm` | none (an empty legend draws no line at all) |

`SimpleTheme` writes `up/down` for `↑↓`. A list prompt leaves room for the legend line when it pages its items.

## Flows
A `Flow` runs the steps of one task in order over a state your app owns, and returns the state. A step is plain app code that usually asks one or more prompts and can do anything else in between (call a server, show a spinner, print a message):
```rust
use paloma::{flow, Done, Flow, Input, Message, Password};

#[derive(Default)]
struct SignUp { email: String, password: String, port: Option<u16>, custom: bool }

let state = Flow::new(SignUp::default())
    .title("Sign up")
    .keep_last(2)              // only the last two answered steps stay while it runs
    .done(Done::Collapse)      // one summary line at the end: `✔ Sign up · 4 steps`
    .step("email", |s| {
        // The state keeps earlier answers, so going back can prefill them.
        s.email = Input::new().prompt("Email").prefill(s.email.clone()).interact()?;
        Ok(())
    })
    .step("password", |s| {
        s.password = Password::new().prompt("Password").interact()?;
        if !sign_in(&s.email, &s.password) {
            return flow::back_to("email", Message::error("Wrong email or password"));
        }
        Ok(())
    })
    .step_if(|s| s.custom, "port", |s| {
        s.port = Some(Input::new().prompt("Port").default(443).interact()?);
        Ok(())
    })
    .run()?;
```
* **Steps.** `.step(name, |state| …)` adds a step; `.step_if(condition, name, |state| …)` one that runs only while `condition(&state)` holds. A step returns `Ok(())` to move on. Any error other than the two below ends the flow with that error (the screen stays as it is).
* **Step counter.** Prompts in a flow show the step number and the total (`01/04`), drawn by the theme after the question. A step's number is its position among the steps that run, and the total counts the steps whose condition holds for the state when the step starts, so it grows when a conditional step joins (`04/04`, then `05/05`). Answered lines have no counter. `.counter(false)` turns it off for one flow, `Options::flows(|f| f.counter(false))` for the app.
* **`Esc` goes back.** Inside a flow, `interact()` returns `Err(Error::Back)` on `Esc` (and on `q` where `interact_opt` takes it: `Select`, `MultiSelect`, `Sort`, `Confirm`), so a step goes back with plain `?`. The flow goes back to the nearest earlier step that asked something (steps answered by a preset or unattended mode are skipped, since they would only answer again); on the first step, or with no such step, `Esc` cancels the flow and `run()` returns `Err(Error::Cancelled)`. `.esc_back(false)` (or `Options::flows`) turns this off: `Esc` then does nothing in `interact()`, as outside a flow. `interact_opt()` keeps returning `Ok(None)` everywhere, for steps that handle `Esc` themselves.
* **`back_to`.** `return flow::back_to("email", Message::error("…"))` ends the step and jumps back to an earlier step by name (or `flow::back_to("email", None)` without a message); the current step's own name runs it again. The message shows in the row of the first prompt the target step opens, as a validation error. A name that matches no earlier step that ran is an `io::ErrorKind::InvalidInput` error from `run()`.
* **Going back erases** what the target step and every step after it printed, so the screen never shows an answer twice.
* **`^C`** in a prompt cancels the whole flow, like `Esc` on the first step: by default paloma prints the `cancel_message` (if set) and exits with code 130; with `cancel_exits(false)`, `run()` returns `Err(Error::Cancelled)`. The screen stays as the prompt left it.
* **Presets and unattended mode** work per step, so a flow run with `--yes` asks nothing.
* **What stays on screen.** `.step_done(Done)` sets what each answered prompt leaves, unless the prompt sets its own `done` (the cascade is: the prompt's `done`, the flow's `step_done`, then the prompt's `Options` group and `prompts`). `.keep_last(n)` keeps only the output of the last `n` finished steps that printed something while the flow runs (by default all of it stays). `.done(Done)` sets what the whole flow leaves when it finishes: everything (`Keep`, the default), one summary line (`Collapse`, drawn by the theme's `format_flow_summary` with the flow's `.title(..)`; without a title it acts like `Erase`) or nothing (`Erase`). `Esc` on the first step erases the flow's output when its `done` is `Erase` or `Collapse`.
* **One flow at a time.** `run()` while another flow runs (a flow inside a step) returns an `io::ErrorKind::InvalidInput` error.

`Options::flows(|f| …)` holds `counter`, `esc_back`, `step_done` and `done` for every flow; a flow's own setter wins, then `Options::flows`, then the defaults (counter on, `Esc` back on, no `step_done`, `done` `Keep`). `.theme(&t)` draws the summary line with `t` instead of the app theme.

**What a flow can erase.** paloma cannot read the screen back, so while a flow runs every lasting line paloma writes to a terminal is recorded with the step that wrote it: answered and kept prompts, messages, output blocks, and a task's step and result lines. A stream that is not a terminal is not recorded (it never shows on screen). Output written past paloma (your own `println!`) is not recorded, and erasing would then leave it or miscount: inside a flow, print through paloma (`paloma::info(..).print()`, a `Section`, …), or call `flow::printed(text)` after printing (`keep_last` prints such text again on stderr). Rows that scrolled out of the terminal cannot be erased.

The counter is drawn by the theme from the `step` field of the question views (`PromptView`, `InputPromptView`, `ConfirmPromptView`, `FuzzySelectPromptView`; `None` outside a flow or with the counter off). The default layouts put it right after the question: `step_gap` spaces (default 2), then `02/05` in `step_style` (default the `subtle` role), before the suffix, the default and the choices. The summary line uses the `input` prompt look: the answered mark, the title in `answered_label_style`, the separator, then `5 steps` in `value_style`, where `steps` is the `flow_steps_word` token. See `examples/flow.rs`.

## Messages
Status messages: `paloma::description / hint / info / success / warning / error(text)` (or `Message::error(..)` and friends) return a builder; `.print()` writes it.
```rust
use paloma::Message;

paloma::error("Port 3000 is already in use")
    .nest(Message::hint("Stop the other server or pick another port"))
    .print()?;
```
`SimpleTheme` prints words (`error: …`, `  hint: …`), `ColorfulTheme` coloured glyphs (`✘ …`). `.nest(child)` draws a message indented under its parent, at any depth. A message's look comes from the `messages` tokens (`MessageTokens`: a `base` and one group per status, each with `glyph`, `word`, `word_gap` (spaces between the word and the text, default 1), and the `glyph_style`, `word_style` and `text_style`). `.stream(Stream::Stdout)` and `.theme(&t)` change one message; `.print_on(&term)` writes to a given terminal. See `examples/messages.rs`.

## Output blocks
Ready-made blocks for what a command prints besides messages. Each has `.stream(..)`, `.theme(&t)`, `.print()` and `.print_on(&term)`.
```rust
use paloma::{KeyValue, Message, Panel, PanelRow, Table};

paloma::section("Setup").print()?;                                   // # Setup
KeyValue::new().row("State", "Connected").row("Server", "es-bcn-01").print()?;
Table::new(["Server", "Load"]).row(["us-nyc-01", "42%"]).print()?;
Panel::new("Setup complete")
    .row(PanelRow::from(Message::success("Signed in")).trailing("1.8s"))
    .row("Connect on startup: On")
    .print()?;
```
`KeyValue` and `Table` print to stdout by default (they are usually results), `Section` and `Panel` to stderr; options `blocks`, `section`, `key_value`, `table`, `panel` change that. Their look comes from the `blocks` tokens (`BlockTokens`: `space_before`, `gap`, `min_width` / `max_width` for panels, and title, key, value, header, cell, border and trailing styles), with a `base` for every block and one group per block.

`paloma::text::link(url)` is an OSC 8 hyperlink on a terminal (plain text elsewhere; `.text(..)` for other link text) and `paloma::text::gradient(text, from, to)` colours text along an RGB gradient where 24-bit colour is available. Both are `Display`. See `examples/blocks.rs`.

## Tasks
A spinner for work that takes a while. `paloma::task(label).start()?` draws the label with a turning spinner and the elapsed time; `step` prints a nested line above it, `label` changes the running label, and `success` / `warning` / `fail` (or `finish(status, text)`) stop the spinner and print the result line. `clear()` stops it and prints no result; dropping a `RunningTask` without finishing does the same.
```rust
let task = paloma::task("Installing dependencies...").start()?;
task.step("Resolved 214 packages")?;
let elapsed = task.success("Installed dependencies")?;   // returns the Duration
```
`SimpleTheme` prints `success: Installed dependencies  1.8s` and steps as `    - Resolved 214 packages`, with an ASCII spinner; `ColorfulTheme` uses a Unicode spinner and glyphs. `.theme(t)` takes the theme by value (the spinner redraws from another thread), `.stream(..)` picks the stream, and `.start_on(&term)` draws on a given terminal.

Motion (the turning spinner) is on when the stream is a terminal and the `CI` environment variable is not set; `Options::live(|l| l.motion(true))` or `.task(..)` overrides `CI` but never turns motion on for a pipe or a file. Without motion nothing is drawn at `start`, and each step and the result line are printed once. Other options: `frame_ms` (default 80) and `stream` (default stderr), in the `live` family group and the `task` group.

One live component runs at a time; a second `start` returns an error. Print only through the task while it runs (`step`, not `println!`). A prompt shown while a task runs pauses the spinner and brings it back below the answer, so a step can ask something in the middle of work.

paloma installs no signal handler. The spinner hides the cursor, so call `paloma::terminal::restore()` from your own Ctrl+C handler before exiting; it erases the spinner line and shows the cursor, and is safe when nothing is running:
```rust
ctrlc::set_handler(|| {
    paloma::terminal::restore();
    std::process::exit(130);
})?;
```
Finishing the task, dropping it and a panic that unwinds all give the cursor back. A task's look comes from the `live` tokens (`LiveTokens`: `spinner_style`, `label_style`, `step_style`, `nested_style`, `elapsed_style`, `step_indent` and `show_elapsed`, with a `base` and a `task` group) and the `spinner` and `nested` glyphs; themes draw it in `format_task` and `format_task_step`. See `examples/tasks.rs`.

## Progress bars
A bar for work with a known size. `paloma::progress(label, total).start()?` shows the bar at 0 on the same live region as a task; `set(value)` and `inc(by)` move it, `label` changes the label, and it ends one of three ways: `keep()` leaves the bar on screen as a lasting line, `success` / `warning` / `fail` (or `finish(status, text)`) replace it with a result line drawn like a task's, and `clear()` removes it and leaves nothing (dropping a `RunningProgress` without finishing does the same). Each returns the elapsed time.
```rust
let bar = paloma::progress("Downloading...", 480).start()?;
for chunk in chunks {
    // ... fetch the chunk ...
    bar.inc(chunk.len() as u64)?;
}
bar.success("Downloaded 480 KB")?;   // or bar.keep()? to leave the bar itself
```
The value is clamped to the total, and `set` with a lower value is ignored, so the bar never moves backwards. `value()` is the real value. On screen the bar eases toward it: each frame (40 ms) it moves a quarter of the distance, and at least a quarter of a cell, so a burst of `set` or `inc` calls costs nothing and a jump glides. `keep` and the result line are drawn at the real value, and `keep` has no head glint. A `total` of 0 draws an empty bar with `0%`, and `keep` then shows it full at `100%`. A bar has no `step` (put what is happening in the label; a [task](#tasks) is for step lines).

Everything else works as for a task: the app theme draws it (`.theme(t)` takes the theme by value), `.stream(..)` and `.start_on(&term)` are the same, motion is on only for a terminal when `CI` is not set (otherwise nothing is drawn until `keep` or the result line, which print once), only one live component runs at a time, a prompt shown meanwhile pauses the bar and brings it back below the answer, and `paloma::terminal::restore()` in a Ctrl+C handler erases it and shows the cursor. Options are in the `live` family group and the `progress` group: `motion`, `frame_ms` (default 40, where a task's is 80) and `stream`.

The look is the `live` tokens, with a `progress` group (and the family `base`):

| Token | Default | |
| --- | --- | --- |
| `bar_width` | 30 | cells in the bar |
| `bar_indent` | 0 | columns before the bar |
| `head_width` | 0 | cells of the glint at the end of a running bar's fill (none by default) |
| `fill_style` | `accent` | the filled cells (without the head) |
| `head_style` | `accent_strong` | the glint |
| `track_style` | `faint` | the unfilled cells |
| `label_style` | `muted` | the label (shared with tasks) |
| `percent_style` | `subtle` | the percentage |
| `show_percent` | true | whether `  50%` follows the label |
| `show_elapsed` | false | whether the elapsed time follows (true by default for a task) |
| `elapsed_style` | `subtle` | the elapsed time (shared with tasks) |

The cells are the `bar_fill` and `bar_track` glyphs, which must differ: without colour (`NO_COLOR`, `SimpleTheme`) nothing else tells them apart. `SimpleTheme` draws `===============--------------- Downloading...  50%` and `ColorfulTheme` `███████████████░░░░░░░░░░░░░░░ Downloading...  50%`. A design system that draws the fill and the track as one glyph in two colours sets both glyphs to the same one, as Ultraviolet does (`bar_fill` and `bar_track` `━`, 32 cells, a two-cell glint):
```rust
paloma::set_theme(ColorfulTheme.restyle(|t| t
    .glyphs(|g| g.bar_fill("━").bar_track("━"))
    .live(|l| l.progress(|b| b.bar_width(32).head_width(2)))));
```
The default layout is `bar_indent` spaces, the bar, a space, the label and, if `show_percent`, two spaces and `value * 100 / total` as an integer, then the elapsed time if `show_elapsed`. Each part (the fill, the head, the track, the label, the percentage) is painted as one span, and an empty part writes nothing. Themes draw a bar in `format_progress`, from a `ProgressView` (`text`, `state`: `Running`, `Kept` or `Finished(status)`, `value` (the eased one while running), `total` and `elapsed`); rate and time left are not built in, but a theme can compute them from `elapsed`, `value` and `total`. See `examples/progress.rs`.

## Themes
`SimpleTheme` (plain, the default) and `ColorfulTheme` (colour and Unicode glyphs) are the same default layout drawn with two token presets. Change any part of either with `restyle` (below):
```rust
use paloma::{console::Style, theme::{ColorfulTheme, Theme}};
let theme = ColorfulTheme
    .restyle(|t| t.prompts(|p| p.base(|b| b.value_style(Style::new().yellow().dim()))));
```
Migrating from 0.12: `report(bool)` is `done(Done)` (`report(false)` is `Done::Erase`), `clear(bool)` on the list prompts is `done(Done::Keep)` or `done(Done::Erase)`, and `GeneralOptions::report` and `PromptOptions::report` are `prompts().done`; `Error` is `#[non_exhaustive]` and has `Cancelled` and `NoDefault`, so an exhaustive `match` needs a wildcard arm, and `^C` at a prompt is now `Err(Error::Cancelled)` instead of the process receiving SIGINT; `Esc` in `interact_opt` of `Confirm` erases the prompt instead of leaving a skipped line; `ItemView::new` takes `description` and `disabled` after `matches`; `Answer` has a `Cancelled` variant; `Select::item`, `MultiSelect::item` and the other item methods take `impl IntoItem` instead of `ToString`. Validator errors need `IntoMessage` instead of `ToString` (every `Display` type has it); `PromptView::new`, `InputPromptView::new`, `ConfirmPromptView::new` and `FuzzySelectPromptView::new` take `invalid` as their last argument, and `FuzzySelectPromptView::new` takes `placeholder` after `cursor`. Since flows, the four question views (`PromptView`, `InputPromptView`, `ConfirmPromptView`, `FuzzySelectPromptView`) also take `step` as their last `new` argument (`None` outside a flow), `Error` has `Back` and `BackTo`, and `interact()` inside a `Flow` returns `Err(Error::Back)` on `Esc`. `ColorfulTheme` has no fields any more and is written like `SimpleTheme` (`ColorfulTheme::default()` still compiles). `ColorfulTheme { values_style: s, .. }` becomes `ColorfulTheme.restyle(|t| t.prompts(|p| p.base(|b| b.value_style(s))))`; the other fields map to prompt tokens the same way (`prompt_style` → `label_style`, `prompt_prefix` → the `prompt` glyph and `mark_style`, `prompt_suffix` → `prompt_suffix` and `suffix_style`, `success_prefix` → `answered` and `answered_mark_style`, `success_suffix` → `answered_separator` and `separator_style`, `defaults_style` → `default_style`, `hint_style` → `hint_style`, `active_item_style` / `inactive_item_style` → `active_style` / `item_style`, `active_item_prefix` → `focus` and `focus_style`, `checked_item_prefix` / `unchecked_item_prefix` → `checked` / `unchecked` and their styles, `fuzzy_cursor_style` → `cursor_style`, `fuzzy_match_highlight_style` → `match_style`, `error_prefix` / `error_style` → the `error` message tokens). Prompt hooks changed too (see below).
Set one theme for the whole app with `paloma::set_theme(theme)` (the theme must be `Send + Sync`). Calling it again replaces the theme. A prompt given `.theme(&t)` keeps `t` and ignores the app theme. See `examples/set_theme.rs`.

For a full design system in practice, see `examples/ultraviolet/`: a fake VPN CLI whose `UltravioletTheme` sets its palette, glyphs and its prompt, message, block and live tokens, and overrides only the hooks whose layout tokens cannot express (its question line with the flow's step counter from the view, the input on its own line, its confirm), installed once with `set_theme` (`cargo run --example ultraviolet --all-features -- setup`; `help` lists every command, option and environment variable, as in the other two examples). Its `setup` command is a `Flow`, and its sections, messages, tables, panels, tasks, progress bar and `--yes` answers are paloma's own components and presets; its only drawing of its own is the Serrallonga icon and the wordmark, the app's brand art (it has no `ui.rs`).

`examples/loam/` is the opposite design, a cozy garden journal (warm earth colours, floral glyphs, dotted rules, lowercase copy; see [docs/design/loam.md](design/loam.md)), and it draws nothing by hand: it has no `ui.rs`, `LoamTheme` implements only `tokens()` (no `format_*` hook), and every prompt, flow, message, block, link, gradient, task and progress bar it shows is paloma's own. It uses every prompt, both confirm layouts, presets and unattended runs (`--yes`), flows with `back_to`, `Esc` back, `step_if` and `keep_last`, the message row (`on_change` with and without `change_delay`, `pending`), legends, rich items, history, completion, the editor, tasks (steps, every result, a prompt during a task), progress bars (sowing, watering and fetching the seed catalogue, each ending in a result line) and the cancel defaults (`cargo run --example loam --all-features -- init`; `loam help` lists everything; `--plain` draws the same app with `SimpleTheme`).

`examples/lull/` is the third, a sleep schedule tracker saved to one JSON file (`cargo run --example lull --features fuzzy-select -- setup`; `today`, `park`, `log`, `week`, `forget` and `help` follow; see [docs/design/lull.md](design/lull.md)). Its point is the opposite of Loam's: it shows paloma's defaults everywhere. It sets no option, `done`, legend or per-prompt theme, and calls only the content side of the prompts, a `Flow` (`setup`, `log`), messages, a `KeyValue` and a `Table`. The only things it styles are two `ColorfulTheme.restyle(..)` themes, a day one and a night one, and it chooses between them once at start-up from the user's own sleep schedule: the night theme from the wind-down until the wake time. `LULL_NOW=YYYY-MM-DDTHH:MM` previews another time of day and `LULL_FILE` picks the file.

Every theme draws with design tokens (`paloma::theme::Tokens`): a `Palette` of colour roles (`text`, `subtle`, `accent`, `success`, `warning`, `error`, …), `Glyphs`, a `Layout`, and component tokens such as `MessageTokens` (a `base` style for every status, overridden per status). `restyle` changes the tokens of any theme without implementing the trait:
```rust
use paloma::{console::Style, theme::{ColorfulTheme, Theme}};
paloma::set_theme(ColorfulTheme.restyle(|t| t
    .palette(|p| p.info(Style::new().magenta()))
    .glyphs(|g| g.warning("▲"))
    .messages(|m| m.error(|e| e.word(Some("error:"))))));
```
Presets: `Tokens::plain()` (`SimpleTheme`) and `Tokens::colorful()` (`ColorfulTheme`). `paloma::terminal::capabilities(Stream::Stderr)` reports whether the terminal shows colours and Unicode and its colour depth, to choose tokens from.

Prompts draw with `PromptTokens`: a `base` `PromptStyle` and one group per prompt (`confirm`, `input`, `password`, `select`, `multi_select`, `sort`, `fuzzy_select`). A `PromptStyle` also holds the tokens of [items](#items) (`item_description_style`, `item_description_gap`, `disabled_style`) and the [legend](#key-legends) (`legend`, `legend_back`, `legend_separator`, `legend_key_style`, `legend_style`), the [flow](#flows) counter and summary (`step_style`, `step_gap`, `flow_steps_word`), besides glyph overrides (`mark`, `answered_mark`, `suffix`, `separator`, `focus`, `checked`, `unchecked`, `cursor`, `mask`, falling back to the glyphs of the same purpose), words (`choices`, `choices_yes`, `choices_no`, `yes`, `no`, `yes_button`, `no_button`, `button_brackets`, `default_brackets`, `hidden_value`, `empty_value`), spacing (`gap`, `item_indent`, `button_indent`, `show_default`) and a style for every part (`label_style`, `value_style`, `active_style`, `paging_style`, …, falling back to palette roles):
```rust
use paloma::{console::Style, theme::{SimpleTheme, Theme}};
paloma::set_theme(SimpleTheme.restyle(|t| t
    .glyphs(|g| g.focus("→"))
    .prompts(|p| p
        .base(|b| b.item_indent(2).active_style(Style::new().bold()))
        .confirm(|c| c.yes("Sure").no("Nope")))));
```

For a fully custom look, implement `paloma::theme::Theme`; every `format_*` method has a default, so override only what you need. Every hook takes `(f, ctx, view)`: the `Context` holds the tokens (read them from there, so `restyle` keeps working), the stream and its capabilities, and the view holds what to draw. Defaults are public in `paloma::theme::defaults` so an override can wrap them. Prompt hooks: `format_prompt` (the question line of select, multi select and sort, with the paging counter), `format_input_prompt` (input and password, up to where the typed text goes; paloma then draws the text, the mask or the placeholder and places the cursor), `format_confirm_prompt` and `format_confirm_buttons`, `format_fuzzy_select_prompt`, `format_item` (an item of any list prompt; the view says which prompt via `PromptKind`), `format_prompt_message` (a line of the [message row](#message-row)), `format_legend` (the [legend](#key-legends) line, from a `LegendView` of `(key, action)` pairs), `format_answered` (the line any answered prompt leaves, with an `Answer`) and `format_flow_summary` (the line a [flow](#flows) leaves with `Done::Collapse`, from a `FlowSummaryView`). The question views carry `invalid` while the row shows a validation error, and `step` (the [flow](#flows) step counter) inside a flow. To draw an answered line yourself (for example for a value given as a command-line flag), call `format_answered` with `AnsweredView::new(PromptKind::Input, "Name", Answer::Text("Ada"))`.

More runnable samples live in `examples/` (`cargo run --example wizard`).

## Options
Themes decide how prompts look; options decide how they work. Set them once, next to `set_theme`:

```rust
paloma::set_options(
    paloma::Options::new()
        .prompts(|p| p.done(Done::Collapse).legend(true))  // every prompt
        .confirm(|c| c.choices_as_buttons(true).on_shortcut(false))
        .password(|p| p.mask(true).result_len(8))
        .select(|s| s.done(Done::Erase))                   // overrides prompts for Select
        .messages(|m| m.stream(Stream::Stderr))            // every message
        .success(|s| s.stream(Stream::Stdout)),            // overrides messages for success
);
```

A setting is resolved in this order: the component's own builder method (`Confirm::new().choices_as_buttons(..)`), its member group in `Options`, its family group (`prompts`, `messages`), then the built-in default. Prompt groups: `confirm`, `password`, `input`, `select`, `multi_select`, `sort`, `fuzzy_select`; message groups: `description`, `hint`, `info`, `success`, `warning`, `error`; block groups: `section`, `key_value`, `table`, `panel` (family `blocks`); live groups: `task` and `progress` (family `live`); `flows` holds the settings of every [flow](#flows) (`counter`, `esc_back`, `step_done`, `done`). `general` holds the settings of the whole app: `unattended` (see [Presets and unattended runs](#presets-and-unattended-runs)). Prompt settings are `done` and `legend` on every prompt group, plus the ones of each prompt (`choices_as_buttons`, `mask`, …). Input and password errors (validation, parsing, password mismatch) are drawn in the [message row](#message-row) under the input and disappear once the value changes.
