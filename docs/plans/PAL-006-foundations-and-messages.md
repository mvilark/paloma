# PAL-006: Foundations and messages

Roadmap item: [PAL-006 in ROADMAP.md](../../ROADMAP.md). Umbrella plan: [PAL-004, Phase 1](PAL-004-ui-framework.md#pal-006--phase-1-foundations-and-messages).

## Goal

Give paloma its first non-prompt component, `Message`, and the foundations every later component builds on: one settings mechanism for options and tokens, a small terminal layer, design tokens, a render context for theme hooks, and `restyle`. The foundations land with this first real user so nothing is built on speculation.

## Proposed API / behavior

```rust
use paloma::{Message, Options, theme::{ColorfulTheme, Theme}, terminal::Stream};

// Out of the box: SimpleTheme prints like rustc / cargo.
paloma::error("Port 3000 is already in use")
    .nest(Message::hint("Stop the other server or pick another port"))
    .print()?;
// error: Port 3000 is already in use
//   hint: Stop the other server or pick another port

// Restyle any theme without implementing the trait.
paloma::set_theme(ColorfulTheme::default().restyle(|t| t
    .glyphs(|g| g.warning("▲"))
    .messages(|m| m.error(|e| e.word(None)))));

// Behaviour: base family, then one status, then the call.
paloma::set_options(Options::new()
    .messages(|m| m.stream(Stream::Stderr))
    .success(|s| s.stream(Stream::Stdout)));
paloma::success("Installed 214 packages").print()?;
```

- **`Message`** (`src/output/message.rs`): a status and a text, nested child messages, an optional own theme and stream. Constructors `Message::new(Status, text)`, `Message::description / hint / info / success / warning / error`; crate-root shortcuts `paloma::description / hint / info / success / warning / error` return the builder. `.nest(child)`, `.stream(Stream)`, `.theme(&t)`, `.print()` (stream from options, stderr by default) and `.print_on(&Term)`, both returning `paloma::Result`. Each message is one line; children are drawn indented under their parent, at any depth.
- **`Status`**: `Description`, `Hint`, `Info`, `Success`, `Warning`, `Error` (`#[non_exhaustive]`), shared with later components.
- **Options**: new family groups `prompts` (`report`) and `messages` (`stream`), and one group per status (`description`, `hint`, `info`, `success`, `warning`, `error`). Resolution: the call, the status group, the family group, then the default. `general().report` is deprecated in favour of `prompts().report`; prompts resolve `report` as prompt, its group, `prompts`, then `general`.
- **Terminal** (`paloma::terminal`): `Stream` (`Stderr`, `Stdout`), `Capabilities` (`is_tty`, `colors`, `unicode`, `width`, `depth`) from `capabilities(Stream)`, and `ColorDepth` (`Basic`, `Ansi256`, `TrueColor`) from `COLORTERM` / `TERM`. Read-only, so apps and themes can choose a palette. (The cursor guard moves to PAL-008, where its first user is.)
- **Tokens** (`paloma::theme`): `Tokens` = `Palette` (colour roles `text`, `muted`, `subtle`, `faint`, `accent`, `accent_strong`, `success`, `info`, `warning`, `error`), `Glyphs` (`description`, `hint`, `info`, `success`, `warning`, `error`; `Glyphs::unicode()`, `Glyphs::ascii()`, `Glyphs::none()`), `Layout` (`indent`) and component tokens `MessageTokens` (a `base` `MessageStyle` plus one per status; fields `glyph`, `word`, `glyph_style`, `word_style`, `text_style`, all optional, falling back from status to base to the semantic tokens). Presets `Tokens::plain()` (no colour, words like `error:`) and `Tokens::colorful()` (glyphs and colours matching `ColorfulTheme`). Private fields, closure setters, getters.
- **Theme**: `fn tokens(&self) -> Cow<'_, Tokens>` (plain by default; `ColorfulTheme` returns colorful), `fn format_message(&self, f, ctx: &Context, view: &MessageView)`, and `fn restyle(self, |Tokens| -> Tokens) -> Restyled<Self>`. Default bodies live in `paloma::theme::defaults` so overrides can wrap them. `Context` carries the tokens, stream and capabilities, and `ctx.paint(&style, text)` styles for the right stream. `MessageView` (`#[non_exhaustive]`): `status`, `text`, `depth`. `MessageTokens::resolve(status, &Tokens)` gives the final `MessageLook`.
- **Validation errors**: the default `Theme::format_error` draws an error message through the tokens. `SimpleTheme` output stays `error: …`; `ColorfulTheme` keeps its own override until PAL-009.

Nothing existing breaks except the deprecation warning on `general().report`.

## Implementation steps

1. `src/settings.rs`: crate-private `settings!` macro (optional fields, documented setters, fallback merge); existing option groups move onto it.
2. `src/terminal.rs`, `src/theme/tokens.rs`, `src/theme/context.rs`, `src/theme/defaults.rs`, `src/theme/restyle.rs`; `Theme` gains `tokens`, `format_message`, `restyle`.
3. `src/output/message.rs`; re-exports in `lib.rs`.
4. Tests: unit tests for resolution, nesting, restyle and options; message snapshots for both built-in themes in `tests/themes.rs`; existing snapshots unchanged.
5. Docs: `docs/USAGE.md` (Messages, Tokens and restyle), `DEVELOPMENT.md` layout, an example `examples/messages.rs`.

The Ultraviolet example moves onto messages later, after all planned phases (client's call, 2026-10-11).

## Open questions / risks

- `Restyled` must forward every `Theme` method to the wrapped theme; a test checks a restyled `ColorfulTheme` draws prompts exactly like the original.

## Status notes

- 2026-10-11: Plan written; implementation on branch `claude/dazzling-galileo-dqa12c`.
- 2026-10-11: Implemented. Differences from the plan: the cursor guard waits for PAL-008 (no user yet); `ColorfulTheme` keeps its own `format_error`, which looks the same as its error message. Golden tests: prompt snapshots unchanged; new message snapshots for both built-in themes; a restyled `ColorfulTheme` draws every prompt hook and message exactly like the original.
