# Loam design

Loam is the look of a cozy garden journal, the second example app ([PAL-020](../plans/PAL-020-loam-example.md)). It is the opposite of [Ultraviolet](ultraviolet.md) on purpose: Ultraviolet is one electric hue on near-black with hard geometric glyphs and terse capitalised copy; Loam is a warm earth palette in several hues, floral glyphs, lowercase warm sentences and a lot of air.

The whole design is paloma tokens. `examples/loam/theme.rs` builds `Tokens` and implements nothing else of `Theme`, and the app has no UI code (no `ui.rs`, no escape codes, no styled text), so this guide is what the tokens say, not what some drawing code does.

## Principles

- **Several hues, one warmth.** Moss, clay, honey, rain and parchment sit together the way soil, leaves and weather do. No hue shouts: nothing is bold except the focused button, nothing blinks except the bar cursor.
- **Soft, not sharp.** Glyphs are flowers and arcs, rules are dotted, corners are single dots.
- **Colour sits on the glyph.** A status tints its glyph; the sentence after it stays `text`. Hints are a word (`tip`) in italic `subtle`, never a glyph.
- **Air.** One blank line before sections and panels. Prompt rows, items and buttons sit two columns in.
- **Quiet answers.** An answered prompt shrinks to `❀ question — answer`, with the mark and question dim and the answer in `text`.

## Colour

Two palettes, **night** (the default, for dark terminals) and **day** (light terminals). The app never paints the terminal background. Choose with `--night` / `--day` or `LOAM_THEME=day|night`.

| Role | Night | Day | Name | Used for |
| --- | --- | --- | --- | --- |
| `text` | `#EDE3D1` | `#3B2F24` | parchment / espresso | answers, values, message text |
| `muted` | `#B8A992` | `#6B5B4A` | oat | questions once answered, unselected items, keys, table headers |
| `subtle` | `#8C7E6A` | `#8E7D69` | bark | hints, descriptions, placeholders, counters |
| `faint` | `#5A5045` | `#D8CBB6` | soil | the answered separator, box rules, disabled items, elapsed time |
| `accent` | `#9DBF7A` | `#5E8040` | moss | the live mark, focus, spinner, cursor, panel titles |
| `accent_strong` | `#C6E3A0` | `#3F6127` | sprout | the active item, section titles, search matches |
| `success` | `#A3C98F` | `#4E7D4E` | leaf | `✓` |
| `info` | `#9EC3C9` | `#4F7F88` | rain | `◦` |
| `warning` | `#E8B860` | `#A86F12` | honey | `☂` |
| `error` | `#D9785B` | `#B5502F` | terracotta | `✗`, the caret of a refused value |

The wordmark is a gradient from moss to honey (`accent` to `warning`), drawn with `text::gradient`.

### Colour depth

`terminal::capabilities(Stream::Stderr).depth()` picks 24-bit, 256-colour or 16-colour output.

| Role | Night 256 | Day 256 | 16 colours |
| --- | --- | --- | --- |
| `text` | 254 | 236 | default |
| `muted` | 144 | 240 | default |
| `subtle` | 101 | 102 | dim |
| `faint` | 239 | 187 | bright black |
| `accent` | 107 | 64 | green |
| `accent_strong` | 193 | 22 | bright green (night), bold green (day) |
| `success` | 150 | 65 | green |
| `info` | 152 | 66 | cyan |
| `warning` | 179 | 136 | yellow |
| `error` | 173 | 131 | red |

With `NO_COLOR` or output that is not a terminal, nothing is coloured: glyphs and words carry every state.

## Glyphs

Every Unicode glyph is one column wide (a unit test measures them) and has an ASCII fallback, used with `--ascii` and for `TERM=linux` / `TERM=dumb`.

| Purpose | Unicode | ASCII |
| --- | --- | --- |
| prompt mark | `✿` | `*` |
| answered mark | `❀` | `o` |
| prompt suffix | `~` | `~` |
| answered separator | `—` | `-` |
| focus | `❧` | `>` |
| checked / unchecked | `◉` / `◌` | `(x)` / `( )` |
| search cursor | `▏` | `_` |
| password mask | `⁎` | `*` |
| section | `❦` | `#` |
| nested step | `⤷` | `-` |
| info / success / warning / error | `◦` / `✓` / `☂` / `✗` | `i` / `+` / `!` / `x` |
| spinner | `◜ ◝ ◞ ◟` | `. o O o` |
| box: horizontal, vertical, corners | `┈`, `┊`, `·` | `-`, `:`, `.` |
| confirm button brackets | `‹ ›` | `< >` |

## Type

- The question is never bold. The only bold in the app is the focused confirm button (`accent`).
- Italic `subtle` is for everything quiet: hints (`tip`), prompt descriptions, placeholders, item descriptions, key legends and table headers (`muted` italic).
- Strikethrough is for disabled items, in `faint`.
- A search match is underlined in `accent_strong`.
- The terminal cursor of an input is a blinking bar.

## Layout

- Prompt rows (description, validation, legend), list items and confirm buttons are indented 2 columns (`message_indent`, `item_indent`, `button_indent`). A nested message is indented 2 more.
- Sections and panels print one blank line first (`space_before`). Key/value rows and tables are results on stdout and have none.
- Panels are 40 to 60 columns wide, with dotted rules and a dot at each corner.
- A task's steps are indented 2 columns, each after `⤷`.
- The flow step counter keeps paloma's `03/07` form: a `3 of 7` form would need a format token, which paloma does not have. It is drawn in `subtle`, two columns after the question.
- Confirm buttons are the words `yes` and `no`; the focused one is bracketed and bold in `accent`, so the focus shows without colour.

## Voice and copy

- Lowercase, warm, plain sentences: `what do you call this patch of earth?`, `the soil can feed 8 beds this year`.
- Running work ends with `...`; finished work is past tense: `testing the soil...` becomes `the soil is ready`.
- A hint offers a way forward, as a tip: `tip  try fewer beds, or add compost first`.
- Cancelling is kind: `tip  put your trowel down - nothing was changed`, exit code 130.
- The copy is plain ASCII (`...`, `-`) so `--ascii` output is ASCII from end to end; only the theme's glyphs change between the two sets.
- No emoji.

## Motion

A rounded-arc spinner (four frames, 80 ms) in `accent`, with the elapsed time in `faint`. With `CI` set, or when stderr is not a terminal, nothing turns, nothing waits and every duration reads `0.0s`.
