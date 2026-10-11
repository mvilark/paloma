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
    paloma::set_theme(ColorfulTheme::default()); // once, for every prompt
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

Every prompt has `new()` (uses the app theme from `set_theme`, `SimpleTheme` by default), `theme(&dyn Theme)` (a theme for this prompt only), `prompt(..)`, `report(bool)`, and `interact_on(&Term)` to target a specific terminal. Prompts that can be cancelled also have `interact_opt()` / `interact_on_opt()`, which return `None` on Esc/`q` instead of an error.

## Confirm
```rust
let ok: bool = Confirm::new().prompt("Continue?").default(true).interact()?;
let maybe: Option<bool> = Confirm::new().prompt("Continue?").interact_opt()?;
```

`.choices_as_buttons(true)` shows `Yes` / `No` buttons on the line under the question instead of the classic `(y/n)` text: `←`/`→`/`Tab` move the focus, `Enter` picks it, and `y` / `n` jump to a button. The theme draws each button in `format_confirm_button`. `.on_shortcut(false)` makes `y` / `n` only select, so the user hits `Enter` to submit; it works with and without buttons. The prompt is erased after the answer and, with `report(true)`, replaced by the one-line selection.
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
Other options: `prefill`, `allow_empty`, `show_default`, `answered_prompt`, `history(&mut H)` (feature `history`), `completion(&C)` (feature `completion`).

## Select
Returns the chosen index.
```rust
let i = Select::new().prompt("Pick").items(["a", "b", "c"]).default(1).interact()?;
```
Other options: `item`, `max_length` (paging), `clear`. `interact_opt` returns `None` on Esc/`q`.

## MultiSelect
Returns the selected indices.
```rust
let picked: Vec<usize> = MultiSelect::new()
    .prompt("Features")
    .items_checked([("fast", true), ("safe", false)])
    .interact()?;
```

## Sort
Returns the items' indices in the user's new order.
```rust
let order: Vec<usize> = Sort::new().prompt("Rank").items(["x", "y", "z"]).interact()?;
```

## Password (feature `password`, on by default)
```rust
let pw = Password::new()
    .prompt("Password")
    .confirmation("Repeat", "Mismatch")
    .interact()?;
```
Other options: `allow_empty_password`, `mask_char(char)`, `validate` (takes `Fn(&String) -> Result<(), E>`).

While typing, `.mask(true)` echoes the theme's `password_mask_glyph()` for each key press (`.mask_char('•')` picks the character yourself). After the password is confirmed, the one-line result shows a fixed number of mask characters, 8 by default, set with `.result_len(n)`; themes draw it in `format_password_prompt_selection_masked`.

## FuzzySelect (feature `fuzzy-select`)
Type to filter; returns the index into the original items.
```rust
let i = FuzzySelect::new().prompt("Search").items(["apple", "banana"]).interact()?;
```
Other options: `prefill`, `highlight_matches`, `vim_mode`, `max_length`.

## Editor (feature `editor`, on by default)
Opens `$EDITOR`; returns `None` if not saved when `require_save(true)`.
```rust
let text: Option<String> = Editor::new().extension(".md").edit("initial text")?;
```

## History and completion
`BasicHistory::new().max_entries(8).no_duplicates(true)` implements `History`; pass `&mut` via `Input::history`. Implement `Completion::get(&self, input: &str) -> Option<String>` for tab-completion via `Input::completion`. See `examples/history.rs`, `history_custom.rs`, `completion.rs`.

## Messages
Status messages: `paloma::description / hint / info / success / warning / error(text)` (or `Message::error(..)` and friends) return a builder; `.print()` writes it.
```rust
use paloma::Message;

paloma::error("Port 3000 is already in use")
    .nest(Message::hint("Stop the other server or pick another port"))
    .print()?;
```
`SimpleTheme` prints words (`error: …`, `  hint: …`), `ColorfulTheme` coloured glyphs (`✘ …`). `.nest(child)` draws a message indented under its parent, at any depth. `.stream(Stream::Stdout)` and `.theme(&t)` change one message; `.print_on(&term)` writes to a given terminal. See `examples/messages.rs`.

## Themes
`SimpleTheme` (plain, default for `new()`) and `ColorfulTheme` (public styled fields, e.g. `prompt_style`, `values_style`; use struct-update syntax):
```rust
use paloma::{console::Style, theme::ColorfulTheme};
let theme = ColorfulTheme { values_style: Style::new().yellow().dim(), ..ColorfulTheme::default() };
```
Set one theme for the whole app with `paloma::set_theme(theme)` (the theme must be `Send + Sync`). Calling it again replaces the theme. A prompt given `.theme(&t)` keeps `t` and ignores the app theme. See `examples/set_theme.rs`.

For a full design system in practice, see `examples/ultraviolet/`: a fake VPN CLI whose `UltravioletTheme` implements `Theme` directly and is installed once with `set_theme` (`cargo run --example ultraviolet --all-features -- setup`).

Every theme draws with design tokens (`paloma::theme::Tokens`): a `Palette` of colour roles (`text`, `subtle`, `accent`, `success`, `warning`, `error`, …), `Glyphs`, a `Layout`, and component tokens such as `MessageTokens` (a `base` style for every status, overridden per status). `restyle` changes the tokens of any theme without implementing the trait:
```rust
use paloma::{console::Style, theme::{ColorfulTheme, Theme}};
paloma::set_theme(ColorfulTheme::default().restyle(|t| t
    .palette(|p| p.info(Style::new().magenta()))
    .glyphs(|g| g.warning("▲"))
    .messages(|m| m.error(|e| e.word(Some("error:"))))));
```
Presets: `Tokens::plain()` (`SimpleTheme`) and `Tokens::colorful()` (`ColorfulTheme`). `paloma::terminal::capabilities(Stream::Stderr)` reports whether the terminal shows colours and Unicode and its colour depth, to choose tokens from.

For a fully custom look, implement `paloma::theme::Theme`; every `format_*` method has a default, so override only what you need. Hooks that take a `Context` (such as `format_message`) read tokens from it, and their defaults are public in `paloma::theme::defaults` so an override can wrap them.

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

A setting is resolved in this order: the component's own builder method (`Confirm::new().choices_as_buttons(..)`), its member group in `Options`, its family group (`prompts`, `messages`), then the built-in default. Prompt groups: `confirm`, `password`, `input`, `select`, `multi_select`, `sort`, `fuzzy_select`; message groups: `description`, `hint`, `info`, `success`, `warning`, `error`. `general().report` still works but is deprecated in favour of `prompts().report`. Input and password errors (validation, parsing, password mismatch) are drawn on the line under the input and disappear once the user submits again; a theme styles them with `format_error`.
