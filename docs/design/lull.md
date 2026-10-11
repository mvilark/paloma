# Lull design

Lull is a sleep schedule tracker for people whose bedtime drifts because they hyperfocus in the evening, the third example app ([PAL-021](../plans/PAL-021-lull-sleep-tracker.md)). Its point as an example is the opposite of a showcase of options: **the least code that makes a complete app, with paloma's default behaviour everywhere**.

## The allowed-API rule

The app writes content (questions, items, rules, messages) and two themes, nothing else. The example may call only:

- **Theme:** `set_theme`, `ColorfulTheme`, `SimpleTheme`, `Theme::restyle`, the `Palette` and `Glyphs` setters (only in `theme.rs`), and `terminal::capabilities(Stream::Stderr)` with `is_tty` / `unicode` / `depth`.
- **Prompts:** `Select`, `MultiSelect`, `FuzzySelect`, `Input`, `Confirm`, with `new`, `prompt`, `items`, `items_checked`, `default`, `validate`, `description`, `placeholder`, `interact`, and `Item::new(..).description(..)`.
- **Flows:** `Flow::new`, `step`, `step_if`, `run`.
- **Output:** `section`, `info` / `success` / `warning` / `error`, `Message::hint` / `info` with `.nest(..)` / `.print()`, `KeyValue` and `Table`.

It never calls `set_options`, `done`, `legend`, `choices_as_buttons`, `show_default`, `on_change`, `preset`, `with_theme`, `Flow::title` / `counter` / `esc_back` / `keep_last` / `done` / `step_done` / `theme`, `back_to`, tasks, panels, links or gradients, and has no escape codes, `Style` or colour names outside `theme.rs`. `tests/lull.rs` and the grep rules in the plan check this. A gap found this way is recorded in the plan, never worked around.

## The theme follows the schedule

`main.rs` chooses one theme when the program starts, from the user's own schedule and the clock (`LULL_NOW` for previews and tests):

| Phase | When | Theme |
| --- | --- | --- |
| Day | any time outside the night | day, "daylight" |
| Wind-down | from the start of the wind-down until bedtime | night, "moonlight" |
| Night | from bedtime until the wake time | night, "moonlight" |

The night is a plan (`schedule::plan`): `wake`, then `bed = wake - sleep` and `wind_down = bed - wind-down minutes`, all on one timeline, so a night that crosses midnight needs no special case. Two nights are looked at, the one that started yesterday and the one that starts today. With no schedule yet, the default (wake 07:00, 8 h, 60 min wind-down) decides, so running `setup` late in the evening already shows the night theme. A terminal without Unicode gets `SimpleTheme`, and `NO_COLOR` needs no code.

## Themes

Both are `ColorfulTheme.restyle(..)`. Each drops `ColorfulTheme`'s fixed prompt and block styles (`PromptTokens::default()`, `BlockTokens::default()`), so every part of a prompt, section and table takes its colour from the palette, then changes palette roles and three glyphs. Nothing else is set.

### Colour

Colour 256 is also used on 24-bit terminals. On 16 colours each role falls back to a named colour.

| Role | Day "daylight" 256 | Day 16 | Night "moonlight" 256 | Night 16 |
| --- | --- | --- | --- | --- |
| `text` | terminal default | default | 146 | default |
| `muted` | terminal default | default | 103 | default |
| `subtle` | 244 | bright black | 60 | bright black |
| `faint` | 244 | bright black | 238 | bright black |
| `accent` | 214 | yellow | 104 | blue |
| `accent_strong` | 208, bold | yellow, bold | 147 | bright blue |
| `success` | 71 | green | 108 | green |
| `info` | 74 | cyan | 110 | cyan |
| `warning` | 178 | yellow | 180 | yellow |
| `error` | 167 | red | 174 | bright red |

The night theme is deliberately low-contrast and never bold: it is read in a dark room, minutes before sleep. The day theme's only bold is `accent_strong`.

### Glyphs

| Glyph | Day | Night |
| --- | --- | --- |
| prompt | `☼` | `☾` |
| answered | `ColorfulTheme`'s | `⋆` |
| section | `☼` | `✧` |

All three are one column wide, checked by the unit test `glyphs_are_one_column`.

## Sleep rules

These are the app's whole "science". Setup offers only choices the research supports, never free text for the schedule.

| Rule | Value in the app | Basis |
| --- | --- | --- |
| Sleep need | 7, 7.5, 8, 8.5 or 9 hours; default 8 | Adults need at least 7 hours (Watson et al. 2015); 7-9 recommended (Hirshkowitz et al. 2015). |
| Same wake time every day | No weekend schedule exists | Regular timing keeps the body clock stable; irregular sleep is linked to worse sleep and health (Phillips et al. 2017). |
| Wind-down | 30, 45, 60 or 90 minutes; default 60 | A screen-free, work-free buffer before bed; evening light-emitting screens delay sleepiness and the body clock (Chang et al. 2015). |
| Parking the work | The `park` habit and command | Writing a to-do list before bed helped people fall asleep faster (Scullin et al. 2018). |
| Caffeine cut-off | 6, 8 or 10 hours before bed; default 8 | Caffeine 6 hours before bed still cut sleep by about an hour (Drake et al. 2013). |
| Sunrise wake window | Sunrise, rounded to 5 minutes and clamped to 06:00-08:00 | Morning light anchors the body clock; the clamp keeps the time inside common healthy wake times when sunrise is very early or late. With no sunrise (polar day or night) the wake is 08:00. |
| Easing in | The whole night moves 15 minutes earlier every 3 days | Shifting a late schedule gradually rather than all at once is the usual advice for a delayed sleep phase. |

The sunrise is computed offline from the coordinates of the chosen city (the `sunrise` crate), in the computer's time zone.

### References

- Watson NF, Badr MS, Belenky G, et al. Recommended amount of sleep for a healthy adult: a joint consensus statement of the American Academy of Sleep Medicine and Sleep Research Society. *Sleep* 2015;38(6):843-844.
- Hirshkowitz M, Whiton K, Albert SM, et al. National Sleep Foundation's sleep time duration recommendations: methodology and results summary. *Sleep Health* 2015;1(1):40-43.
- Phillips AJK, Clerx WM, O'Brien CS, et al. Irregular sleep/wake patterns are associated with poorer academic performance and delayed circadian and sleep/wake timing. *Scientific Reports* 2017;7:3216.
- Chang AM, Aeschbach D, Duffy JF, Czeisler CA. Evening use of light-emitting eReaders negatively affects sleep, circadian timing, and next-morning alertness. *PNAS* 2015;112(4):1232-1237.
- Scullin MK, Krueger ML, Ballard HK, Pruett N, Bliwise DL. The effects of bedtime writing on difficulty falling asleep: a polysomnographic study comparing to-do lists and completed activity lists. *Journal of Experimental Psychology: General* 2018;147(1):139-146.
- Drake C, Roehrs T, Shambroom J, Roth T. Caffeine effects on sleep taken 0, 3, or 6 hours before going to bed. *Journal of Clinical Sleep Medicine* 2013;9(11):1195-1200.

## Data

One JSON file (`LULL_FILE`, default `~/.lull.json`), written with `serde_json::to_string_pretty`. The shape, the habits and the commands are in the plan. Habit keys and the wake kind are Rust enums, so an unknown value is a read error that names it, never a silent default, and a file that cannot be read is never overwritten.
