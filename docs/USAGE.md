# Using the library

The crate is currently still published and imported as `dialoguer` (see `Cargo.toml`), even though this fork is called "interacteur". Every prompt is a builder: configure it, then call `interact()`. All of them return `dialoguer::Result<T>` (`Error::IO` only), and Ctrl-C / Esc handling is described per prompt below.

```toml
[dependencies]
dialoguer = "0.12"   # default features: editor, password
# optional features: fuzzy-select, history, completion
```

```rust
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

fn main() -> dialoguer::Result<()> {
    let theme = ColorfulTheme::default();
    let name: String = Input::with_theme(&theme).with_prompt("Name").interact_text()?;
    let env = Select::with_theme(&theme)
        .with_prompt("Environment")
        .items(["dev", "staging", "prod"])
        .default(0)
        .interact()?;
    if Confirm::with_theme(&theme).with_prompt(format!("Deploy {name} to #{env}?")).interact()? {
        println!("deploying");
    }
    Ok(())
}
```

Every prompt has `new()` (uses `SimpleTheme`), `with_theme(&dyn Theme)`, `with_prompt(..)`, `report(bool)`, and `interact_on(&Term)` to target a specific terminal. Prompts that can be cancelled also have `interact_opt()` / `interact_on_opt()`, which return `None` on Esc/`q` instead of an error.

## Confirm
```rust
let ok: bool = Confirm::new().with_prompt("Continue?").default(true).interact()?;
let maybe: Option<bool> = Confirm::new().with_prompt("Continue?").interact_opt()?;
```
Other options: `show_default`, `show_hint`, `wait_for_newline`.

## Input
Generic over any `T: Clone + FromStr + ToString`; use `interact_text()`.
```rust
let port: u16 = Input::new().with_prompt("Port").default(8080).interact_text()?;
let email: String = Input::new()
    .with_prompt("Email")
    .validate_with(|s: &String| -> Result<(), &str> {
        if s.contains('@') { Ok(()) } else { Err("needs an @") }
    })
    .interact_text()?;
```
Other options: `with_initial_text`, `allow_empty`, `show_default`, `with_post_completion_text`, `history_with(&mut H)` (feature `history`), `completion_with(&C)` (feature `completion`).

## Select
Returns the chosen index.
```rust
let i = Select::new().with_prompt("Pick").items(["a", "b", "c"]).default(1).interact()?;
```
Other options: `item`, `max_length` (paging), `clear`. `interact_opt` returns `None` on Esc/`q`.

## MultiSelect
Returns the selected indices.
```rust
let picked: Vec<usize> = MultiSelect::new()
    .with_prompt("Features")
    .items_checked([("fast", true), ("safe", false)])
    .interact()?;
```

## Sort
Returns the items' indices in the user's new order.
```rust
let order: Vec<usize> = Sort::new().with_prompt("Rank").items(["x", "y", "z"]).interact()?;
```

## Password (feature `password`, on by default)
```rust
let pw = Password::new()
    .with_prompt("Password")
    .with_confirmation("Repeat", "Mismatch")
    .interact()?;
```
Other options: `allow_empty_password`, `with_mask(char)`, `validate_with` (takes `Fn(&String) -> Result<(), E>`).

## FuzzySelect (feature `fuzzy-select`)
Type to filter; returns the index into the original items.
```rust
let i = FuzzySelect::new().with_prompt("Search").items(["apple", "banana"]).interact()?;
```
Other options: `with_initial_text`, `highlight_matches`, `vim_mode`, `max_length`.

## Editor (feature `editor`, on by default)
Opens `$EDITOR`; returns `None` if not saved when `require_save(true)`.
```rust
let text: Option<String> = Editor::new().extension(".md").edit("initial text")?;
```

## History and completion
`BasicHistory::new().max_entries(8).no_duplicates(true)` implements `History`; pass `&mut` via `Input::history_with`. Implement `Completion::get(&self, input: &str) -> Option<String>` for tab-completion via `Input::completion_with`. See `examples/history.rs`, `history_custom.rs`, `completion.rs`.

## Themes
`SimpleTheme` (plain, default for `new()`) and `ColorfulTheme` (public styled fields, e.g. `prompt_style`, `values_style`; use struct-update syntax):
```rust
use dialoguer::{console::Style, theme::ColorfulTheme};
let theme = ColorfulTheme { values_style: Style::new().yellow().dim(), ..ColorfulTheme::default() };
```
For a fully custom look, implement `dialoguer::theme::Theme`; every `format_*` method has a default, so override only what you need.

More runnable samples live in `examples/` (`cargo run --example wizard`).
