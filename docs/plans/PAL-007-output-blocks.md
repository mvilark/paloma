# PAL-007: Output blocks

Roadmap item: [PAL-007 in ROADMAP.md](../../ROADMAP.md). Umbrella plan: [PAL-004, Phase 2](PAL-004-ui-framework.md#pal-007--phase-2-output-blocks). Builds on [PAL-006](PAL-006-foundations-and-messages.md).

## Goal

Ready-made components for the static output a CLI prints besides messages: a section title, aligned key/value rows, a table, a summary panel with a title and status rows, and two inline helpers (a hyperlink and a colour gradient). Apps stop hand-rolling width maths and escape codes for them.

## Proposed API / behavior

```rust
use paloma::{KeyValue, Message, Panel, PanelRow, Table};

paloma::section("Setup").print()?;                      // # Setup

KeyValue::new()
    .row("State", "Connected")
    .row("Server", "es-bcn-01")
    .print()?;                                          // State    Connected
                                                        // Server   es-bcn-01
Table::new(["Server", "Load"])
    .row(["us-nyc-01", "42%"])
    .row(["es-bcn-01", "23%"])
    .print()?;

Panel::new("Setup complete")
    .row(PanelRow::from(Message::success("Signed in as ada@example.com")).trailing("1.8s"))
    .row("Connect on startup: On")
    .print()?;
// ╭─ Setup complete ──────────────────────╮
// │ ✔ Signed in as ada@example.com   1.8s │
// │ Connect on startup: On                │
// ╰───────────────────────────────────────╯

println!("Docs: {}", paloma::text::link("https://example.com"));
eprintln!("{}", paloma::text::gradient("Ultraviolet", (213, 198, 255), (109, 74, 255)));
```

- **Blocks** share a base (`src/output/block.rs`): `.theme(&t)`, `.stream(Stream)`, `.print()` and `.print_on(&Term)`, all like `Message`. `space_before` blank lines are printed before a block (a token, default 0). Default streams: `Section` and `Panel` on stderr, `KeyValue` and `Table` on stdout (they are usually results); all are options.
- **`Section`** (`paloma::section(title)` or `Section::new`): one title line.
- **`KeyValue`**: `.row(key, value)`, `.rows(iter)`; values aligned to the longest key plus `gap` columns.
- **`Table`**: `Table::new(header)`, `.row(cells)`, `.rows(iter)`; columns padded to their widest cell, `gap` columns apart, header in its own style.
- **`Panel`**: `Panel::new(title)`, `.row(impl Into<PanelRow>)`. `PanelRow::new(text)`, `.status(Status)` (draws that status's message glyph), `.trailing(text)` (right-aligned); `From<&str>`, `From<String>` and `From<Message>`. Width is the terminal width minus 2, clamped to the `min_width` / `max_width` tokens (40 / 64).
- **`paloma::text`**: `link(url)` (an OSC 8 hyperlink, accent and underlined, plain URL when the stream shows no colours; `.text(..)` for other link text; `.stream(..)`), `gradient(text, from, to)` (per-character 24-bit colour when the terminal supports it, otherwise the `.fallback(Style)`, by default the accent-strong role; `.style(Style)` adds attributes such as bold). Both are `Display`.
- **Options**: family group `blocks` (`stream`), member groups `section`, `key_value`, `table`, `panel`.
- **Tokens**: `BlockTokens` (a `base` `BlockStyle` plus one per block) with `space_before`, `gap`, `min_width`, `max_width`, `title_style`, `key_style`, `value_style`, `header_style`, `cell_style`, `border_style`, `trailing_style`, falling back to the palette roles. `Glyphs` gains `section` (`#`) and box parts (`box_h`, `box_v`, corners; Unicode `╭─╮│╰╯`, ASCII `+-|`). Plain preset: `#` and ASCII boxes, no colour.
- **Theme hooks** (Theme v2 shape, defaults in `theme::defaults`, forwarded by `Restyled`): `format_section`, `format_key_value`, `format_table`, `format_panel`, taking `SectionView`, `KeyValueView`, `TableView`, `PanelView`.

All additive.

## Implementation steps

1. `src/output/block.rs` (base, kinds, shared printing; `Message` uses the same printing helper), `section.rs`, `key_value.rs`, `table.rs`, `panel.rs`, `text.rs`.
2. Tokens, options, theme hooks, defaults, `Restyled` forwarding, re-exports.
3. Tests: unit tests per block (alignment, widths, wide characters), block snapshots for both built-in themes in `tests/themes.rs`; existing snapshots unchanged.
4. Docs: `USAGE.md` (Output blocks), `DEVELOPMENT.md` layout; example `examples/blocks.rs`.

## Open questions / risks

- Widths are measured with `console::measure_text_width`, so styled and wide characters align; East Asian ambiguous-width glyphs may still be off by a column in some terminals.

## Status notes

- 2026-10-11: Plan written; implementation on branch `claude/dazzling-galileo-dqa12c`, restarted from `main` after #10 merged.
- 2026-10-11: Implemented. Difference from the plan: in the plain preset titles and headers are not bold (the plain preset has no styling at all, like `SimpleTheme`'s prompts); the colorful preset makes them bold. Links become hyperlinks only on an interactive terminal that shows colours. Existing snapshots unchanged; new block snapshots for both built-in themes; a restyled `ColorfulTheme` draws blocks exactly like the original.
