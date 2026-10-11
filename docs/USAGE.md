# Using the library

This crate, `paloma`, is a fork of dialoguer. Every component is a builder: configure a prompt, then call `interact()`; configure a message, then call `print()`. All of them return `paloma::Result<T>` (`Error::IO` only), and Ctrl-C / Esc handling is described per prompt below.

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

Every prompt has `new()` (uses the app theme from `set_theme`, `SimpleTheme` by default), `theme(&dyn Theme)` (a theme for this prompt only), `prompt(..)`, `report(bool)`, `description(..)`, `on_change(..)` (see [Message row](#message-row)), and `interact_on(&Term)` to target a specific terminal. Prompts that can be cancelled also have `interact_opt()` / `interact_on_opt()`, which return `None` on Esc/`q` instead of an error.

## Confirm
```rust
let ok: bool = Confirm::new().prompt("Continue?").default(true).interact()?;
let maybe: Option<bool> = Confirm::new().prompt("Continue?").interact_opt()?;
```

`.choices_as_buttons(true)` shows `Yes` / `No` buttons on the line under the question instead of the classic `(y/n)` text: `←`/`→`/`Tab` move the focus, `Enter` picks it, and `y` / `n` jump to a button. The theme draws the buttons in `format_confirm_buttons`. `.on_shortcut(false)` makes `y` / `n` only select, so the user hits `Enter` to submit; it works with and without buttons. The prompt is erased after the answer and, with `report(true)`, replaced by the one-line selection.
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
Other options: `item`, `max_length` (paging), `clear`, `validate` (with the chosen index). `interact_opt` returns `None` on Esc/`q`.

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
Other options: `prefill`, `placeholder` (shown while the search is empty), `highlight_matches`, `vim_mode`, `max_length`, `validate` (with the chosen index into the original items).

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

## Messages
Status messages: `paloma::description / hint / info / success / warning / error(text)` (or `Message::error(..)` and friends) return a builder; `.print()` writes it.
```rust
use paloma::Message;

paloma::error("Port 3000 is already in use")
    .nest(Message::hint("Stop the other server or pick another port"))
    .print()?;
```
`SimpleTheme` prints words (`error: …`, `  hint: …`), `ColorfulTheme` coloured glyphs (`✘ …`). `.nest(child)` draws a message indented under its parent, at any depth. `.stream(Stream::Stdout)` and `.theme(&t)` change one message; `.print_on(&term)` writes to a given terminal. See `examples/messages.rs`.

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

## Themes
`SimpleTheme` (plain, the default) and `ColorfulTheme` (colour and Unicode glyphs) are the same default layout drawn with two token presets. Change any part of either with `restyle` (below):
```rust
use paloma::{console::Style, theme::{ColorfulTheme, Theme}};
let theme = ColorfulTheme
    .restyle(|t| t.prompts(|p| p.base(|b| b.value_style(Style::new().yellow().dim()))));
```
Migrating from 0.12: validator errors need `IntoMessage` instead of `ToString` (every `Display` type has it); `PromptView::new`, `InputPromptView::new`, `ConfirmPromptView::new` and `FuzzySelectPromptView::new` take `invalid` as their last argument, and `FuzzySelectPromptView::new` takes `placeholder` after `cursor`. `ColorfulTheme` has no fields any more and is written like `SimpleTheme` (`ColorfulTheme::default()` still compiles). `ColorfulTheme { values_style: s, .. }` becomes `ColorfulTheme.restyle(|t| t.prompts(|p| p.base(|b| b.value_style(s))))`; the other fields map to prompt tokens the same way (`prompt_style` → `label_style`, `prompt_prefix` → the `prompt` glyph and `mark_style`, `prompt_suffix` → `prompt_suffix` and `suffix_style`, `success_prefix` → `answered` and `answered_mark_style`, `success_suffix` → `answered_separator` and `separator_style`, `defaults_style` → `default_style`, `hint_style` → `hint_style`, `active_item_style` / `inactive_item_style` → `active_style` / `item_style`, `active_item_prefix` → `focus` and `focus_style`, `checked_item_prefix` / `unchecked_item_prefix` → `checked` / `unchecked` and their styles, `fuzzy_cursor_style` → `cursor_style`, `fuzzy_match_highlight_style` → `match_style`, `error_prefix` / `error_style` → the `error` message tokens). Prompt hooks changed too (see below).
Set one theme for the whole app with `paloma::set_theme(theme)` (the theme must be `Send + Sync`). Calling it again replaces the theme. A prompt given `.theme(&t)` keeps `t` and ignores the app theme. See `examples/set_theme.rs`.

For a full design system in practice, see `examples/ultraviolet/`: a fake VPN CLI whose `UltravioletTheme` sets its palette, glyphs and prompt tokens and overrides only the hooks whose layout tokens cannot express (a step counter, the input on its own line, its confirm), installed once with `set_theme` (`cargo run --example ultraviolet --all-features -- setup`).

Every theme draws with design tokens (`paloma::theme::Tokens`): a `Palette` of colour roles (`text`, `subtle`, `accent`, `success`, `warning`, `error`, …), `Glyphs`, a `Layout`, and component tokens such as `MessageTokens` (a `base` style for every status, overridden per status). `restyle` changes the tokens of any theme without implementing the trait:
```rust
use paloma::{console::Style, theme::{ColorfulTheme, Theme}};
paloma::set_theme(ColorfulTheme.restyle(|t| t
    .palette(|p| p.info(Style::new().magenta()))
    .glyphs(|g| g.warning("▲"))
    .messages(|m| m.error(|e| e.word(Some("error:"))))));
```
Presets: `Tokens::plain()` (`SimpleTheme`) and `Tokens::colorful()` (`ColorfulTheme`). `paloma::terminal::capabilities(Stream::Stderr)` reports whether the terminal shows colours and Unicode and its colour depth, to choose tokens from.

Prompts draw with `PromptTokens`: a `base` `PromptStyle` and one group per prompt (`confirm`, `input`, `password`, `select`, `multi_select`, `sort`, `fuzzy_select`). A `PromptStyle` holds glyph overrides (`mark`, `answered_mark`, `suffix`, `separator`, `focus`, `checked`, `unchecked`, `cursor`, `mask`, falling back to the glyphs of the same purpose), words (`choices`, `choices_yes`, `choices_no`, `yes`, `no`, `yes_button`, `no_button`, `button_brackets`, `default_brackets`, `hidden_value`, `empty_value`), spacing (`gap`, `item_indent`, `show_default`) and a style for every part (`label_style`, `value_style`, `active_style`, `paging_style`, …, falling back to palette roles):
```rust
use paloma::{console::Style, theme::{SimpleTheme, Theme}};
paloma::set_theme(SimpleTheme.restyle(|t| t
    .glyphs(|g| g.focus("→"))
    .prompts(|p| p
        .base(|b| b.item_indent(2).active_style(Style::new().bold()))
        .confirm(|c| c.yes("Sure").no("Nope")))));
```

For a fully custom look, implement `paloma::theme::Theme`; every `format_*` method has a default, so override only what you need. Every hook takes `(f, ctx, view)`: the `Context` holds the tokens (read them from there, so `restyle` keeps working), the stream and its capabilities, and the view holds what to draw. Defaults are public in `paloma::theme::defaults` so an override can wrap them. Prompt hooks: `format_prompt` (the question line of select, multi select and sort, with the paging counter), `format_input_prompt` (input and password, up to where the typed text goes; paloma then draws the text, the mask or the placeholder and places the cursor), `format_confirm_prompt` and `format_confirm_buttons`, `format_fuzzy_select_prompt`, `format_item` (an item of any list prompt; the view says which prompt via `PromptKind`), `format_prompt_message` (a line of the [message row](#message-row)) and `format_answered` (the line any answered prompt leaves, with an `Answer`). The question views carry `invalid` while the row shows a validation error. To draw an answered line yourself (for example for a value given as a command-line flag), call `format_answered` with `AnsweredView::new(PromptKind::Input, "Name", Answer::Text("Ada"))`.

More runnable samples live in `examples/` (`cargo run --example wizard`).

## Options
Themes decide how prompts look; options decide how they work. Set them once, next to `set_theme`:

```rust
paloma::set_options(
    paloma::Options::new()
        .prompts(|p| p.report(true))                       // every prompt
        .confirm(|c| c.choices_as_buttons(true).on_shortcut(false))
        .password(|p| p.mask(true).result_len(8))
        .select(|s| s.report(false))                       // overrides prompts for Select
        .messages(|m| m.stream(Stream::Stderr))            // every message
        .success(|s| s.stream(Stream::Stdout)),            // overrides messages for success
);
```

A setting is resolved in this order: the component's own builder method (`Confirm::new().choices_as_buttons(..)`), its member group in `Options`, its family group (`prompts`, `messages`), then the built-in default. Prompt groups: `confirm`, `password`, `input`, `select`, `multi_select`, `sort`, `fuzzy_select`; message groups: `description`, `hint`, `info`, `success`, `warning`, `error`; block groups: `section`, `key_value`, `table`, `panel` (family `blocks`); live groups: `task` (family `live`). `general().report` still works but is deprecated in favour of `prompts().report`. Input and password errors (validation, parsing, password mismatch) are drawn in the [message row](#message-row) under the input and disappear once the value changes.
