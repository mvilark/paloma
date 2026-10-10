# Ultraviolet design system

Ultraviolet is the look and behaviour of a prompt-and-flow command-line app: questions asked one at a time, work shown while it happens, a clear result at the end. It is not a full-screen TUI. Output scrolls like any CLI; only the line being worked on redraws.

The character sits between a precise developer tool and a cyberpunk HUD. Precision comes from strict columns, terse copy and a monochrome screen. The HUD feel comes from one electric violet, step counters, `#` section titles and small, constant motion: a braille spinner, a blinking block cursor, one highlight band sweeping the wordmark.

## Principles

- **One hue.** Everything is ink plus violet. `accent` marks where the user is and what is alive; everything else is `fg`, `fg-muted` or `fg-subtle`.
- **Colour means a result.** `success`, `warning` and `error` appear only when something has finished: a task's ✓, a log line's ✕, the outro badge. Prompts, menus, tables and running work never use them.
- **Colour sits on the glyph.** A status tints its glyph, or the glyph on its badge, and nothing else. No status word is printed as a prefix: never `error`, `warning`, `success`, `done` or `info`. The sentence after the glyph stays `fg`.
- **Collapse what is answered.** A finished prompt shrinks to one `◇ Label · value` line so the scrollback reads as a record of decisions.
- **Alive, never busy.** At most one spinner and one shimmer on screen. Nothing animates once it is done.

## Voice and copy

- Sentence case for labels and messages: `Project name`, `Install dependencies now?`. Lowercase for values, commands, keywords and section titles: `# setup`, `next`.
- Prompt labels are nouns or yes/no questions, never instructions: `Template`, not `Please choose a template:`. No trailing colon.
- Running work is present tense with an `-ing` verb and ends with `...`: `Installing dependencies...`. Finished work drops the dots and goes into past tense with what changed: `Installed 214 packages`.
- Status messages carry only their glyph (`✓`, `▲`, `✕`, `•`). The word is never repeated before the message.
- Lead with the fact, then the number: `Port 3000 is already in use.` Times as `1.8s`, sizes as `4.8 MB`, counts as digits.
- Errors say what happened, then a `hint` with the fix as a command the user can paste. No blame, no apology, no exclamation marks.
- No emoji. Glyphs from the set below only.
- "You" for the user, never "I" or "we" for the app.

Real copy, side by side:

| Do | Don't |
| --- | --- |
| `◆ Project name` | `? What would you like to name your project?` |
| `⠹ Installing dependencies...  pnpm  1.2s` | `Installing dependencies, please wait` |
| `✓ Installed 214 packages  1.8s` | `Success!! 🎉 Dependencies installed` |
| `✕ Port 3000 is already in use` | `✕ error  Port 3000 is already in use` |

## Colour

Two themes: **Dark terminal** (the default) and **Light terminal**. The app never paints the terminal background. `term-bg` is the reference ground every text pair is checked against, and each pair also holds on pure `#000000` / `#ffffff`.

Roles:

- `fg` for answers, values and messages. `fg-muted` for answered labels, unselected options, taglines and keys. `fg-subtle` for hints, counters, durations and placeholders.
- `fg-faint` only for decoration: rules, box borders, the progress track, `↳`. Never text, and never the only thing that tells the user something. It is deliberately below 4.5:1 contrast so dividers stay quiet; this is a known, accepted exception to the text-contrast rule above.
- `accent` for the live mark: `◆`, `▸`, `›`, the cursor, the spinner, the progress fill, links, `#`.
- `accent-strong` for emphasis inside a prompt or box: the focused option, the `next` command, box titles.
- `accent-dim` only for decoration (spinner trail, gradient tails). Never text.
- `selection-bg` behind the focused option, padded to the option's width, not the full line.
- `success` (teal), `warning` (amber), `error` (rose) only as described in Principles. On a badge, text is `on-status`.

The palette starts from three colours: `violet-100` `#d5c6ff`, `violet-300` `#a382ff` and `violet-500` `#6d4aff`, extended with `violet-50`, `violet-700` and `violet-900`. Dark terminals use the light end (`accent` = `violet-300`, `accent-strong` = `violet-100`). Light terminals use the deep end (`accent` = `violet-500`, `accent-strong` = `violet-700`). `violet-500` is too dark for text on a dark terminal (3.8:1) and `violet-300` too light on a light one (2.8:1), so that swap matters.

Success is teal rather than green, so it never depends on a red–green split. Every status carries its own glyph (`✓`, `▲`, `✕`), so the meaning holds without colour.

### Choosing the theme

1. `--theme dark|light` or `ULTRAVIOLET_THEME`.
2. An OSC 11 background query (100 ms timeout). Luminance above 0.5 means light.
3. `COLORFGBG` when it is set.
4. Otherwise dark.

### Colour depth

Emit 24-bit colour when `COLORTERM` is `truecolor` or `24bit`. Otherwise fall back:

| Token | Dark: 256 | Light: 256 | 16-colour |
| --- | --- | --- | --- |
| `fg` | 254 | 234 | default fg |
| `fg-muted` | 146 | 60 | default fg |
| `fg-subtle` | 103 | 60 | SGR 2 (dim) |
| `fg-faint` | 60 | 189 | bright black (90) |
| `accent` | 141 | 99 | bright magenta (95) / magenta (35) |
| `accent-strong` | 189 | 55 | bright magenta + bold |
| `success` | 79 | 30 | cyan (36) |
| `warning` | 222 | 94 | yellow (33) |
| `error` | 211 | 161 | red (31) |

`selection-bg`, the wordmark gradient and shimmer are dropped below 256 colours. With `NO_COLOR` set, `TERM=dumb` or output that is not a TTY, print plain text: glyphs and words still carry every state.

## Type

The app uses the user's terminal font. Previews use JetBrains Mono at 14px on a 20px row as a stand-in (`mono`). There are four text styles, all one size:

- `title` (SGR 1, bold): prompt labels, the wordmark, box titles, table headers. One bold run per line.
- `body`: everything else.
- `hint` (SGR 3, italic, in `fg-subtle`): every hint. That means the line under a prompt, option descriptions in lists, key legends, and `cancelled` and `skipped`. Without italic support, colour alone carries it.
- A yes/no prompt shows its shortcuts in the labels, `[Y]es / [N]o`, and prints no key legend.
- `badge`: a status glyph with one space of padding each side, on a status fill. Only in the outro.

Use underline (SGR 4) only for links, and send links as OSC 8 hyperlinks. Use strikethrough only for disabled options.

## Layout

The grid is cells: columns (`col-1` … `col-4`) and rows (`row-1`).

- Column 0 holds the state glyph. Labels start at column 2. Anything that belongs to a prompt (input, options, hints, key hints) is indented `col-2`. Nested task output (`↳`) and code frames are indented `col-4`.
- Lines stop at `measure` (80 columns). Rules and boxes are `min(width − 2, 64)` columns.
- Leave one blank line (`row-1`) between finished sections: banner, prompts, `# setup`, outro. Never two. No blank lines inside a prompt.
- Align values in key/value rows to the longest key + `col-3`. Right-align durations only inside the outro box.

## Glyphs

Unicode glyphs act as the icon set. Each one has a single meaning and an ASCII fallback for `TERM=linux`, legacy Windows consoles and `--ascii`:

| Glyph | Meaning | Colour | ASCII |
| --- | --- | --- | --- |
| `◆` | prompt, active | `accent` | `*` |
| `◇` | prompt, answered | `fg-subtle` | `o` |
| `›` | input caret, step log | `accent` | `>` |
| `▸` | focused option | `accent` | `>` |
| `■` / `□` | checked / unchecked | `accent` / `fg-subtle` | `[x]` / `[ ]` |
| `●` / `○` | chosen, live / pending, unchosen | `accent` / `fg-subtle` | `(*)` / `( )` |
| `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` | running | `accent` | `- \ / -` cycle with a vertical bar |
| `✓` | success | `success` | `+` |
| `▲` | warning | `warning` | `!` |
| `✕` | error | `error` | `x` |
| `↳` | nested output | `fg-faint` | `-` |
| `#` | section title | `accent` | `#` |
| `━` | progress, rule accent | `accent` / `fg-faint` | `=` |
| `╭─╮╰─╯│` | outro box | `fg-faint` | `+`, `-` and a vertical bar |

There is no logo. The wordmark is the app name set in `title`, with a per-character gradient from `wordmark-from` to `wordmark-to`. The app's one picture is the Serrallonga icon: the Catalan bandoler in barretina, with moustache and bandolier, drawn in 8 lines of mixed Unicode symbols from the fixed violet palette. Print it only on `about` screens and first runs, with the wordmark beside it.

## Motion

Motion shows that something is alive. It never decorates.

| What | Timing |
| --- | --- |
| Braille spinner | 10 frames at 80 ms |
| Block cursor blink | 530 ms on, 530 ms off; reset on every keypress |
| Wordmark shimmer | one `shimmer` band, 1.4 s, once on start (previews loop it) |
| Running label shimmer | `fg` band over `fg-muted`, 2.2 s loop, only on the running task |
| Progress head | a two-cell `accent-strong` glint travelling the filled part, 45 ms a cell |
| Progress value | eased toward the real value, never jumping backwards |
| Elapsed time | updates every 100 ms with one decimal |
| New line | appears whole; no typing effect on output |

- Redraw at most 12 times a second, and only the live lines.
- Hide the terminal cursor while a spinner runs. Restore it on exit, on Ctrl+C and on a crash.
- When `CI` is set, output is not a TTY, or `ULTRAVIOLET_MOTION=0`: no spinner, no shimmer, no redraws. Print each task once when it finishes.

## Interaction

- `↑↓` move, `⏎` choose, `␣` toggle, `esc` goes back one prompt, `^C` cancels the whole flow with `◇ … · cancelled` and exit code 130.
- Validate as the user types. Show the error in place of the hint, with `›` turning `error`. Never clear the input.
- Every prompt has a flag (`--name my-app`), and `--yes` accepts defaults so the flow runs unattended.
- Show `01/04` step counters when a flow has three or more prompts.
- Prompts and progress go to stderr. Results (paths, JSON with `--json`) go to stdout.
