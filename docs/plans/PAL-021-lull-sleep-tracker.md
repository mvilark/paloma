# PAL-021: Lull, a sleep schedule tracker example

A third example app, `examples/lull/`: a real, usable sleep schedule tracker for people whose bedtime drifts because they hyperfocus in the evening. It saves to one JSON file. Its point as an example is the opposite of a showcase of options: **the least code that makes a complete app, with paloma's default behaviour everywhere**. The app writes content (questions, items, rules, messages) and two themes, nothing else, and it picks the day or the night theme from the user's own sleep schedule.

## Executor

**Sonnet 5.5, medium effort.**

- **Design left open:** none. Commands, copy, data format, the schedule maths, both themes and the allowed API are fixed below.
- **Risk:** in the time maths, not the terminal. Nights cross midnight, the wake time can follow the sunrise, the schedule can ease in, and the theme switch depends on all three. The plan pins these down as pure functions with named unit tests, which medium effort handles. No terminal state, concurrency or byte-identical output of existing code is touched.
- **Size:** about the size of Loam (PAL-020, Sonnet medium, no rework needed), but with less UI surface and a strict code budget.
- **Verification:** golden transcripts, unit tests and pty runs, so at least medium.

The planner reviews the diff and reruns the checks before anything is pushed. The executor leaves its changes uncommitted, and waits on long checks in the foreground or on a specific PID (never `pgrep -f` on a pattern in its own command line; see TEMPLATE.md).

## Context and decisions

The client's request (2026-10-11), quoted:

> can you plan another app, which doesn't need to have an opposite behaviour than the other examples, which is a sleep tracker that helps people whose sleep schedule is inconsistent because they hyperfocus in the evenings? this one sould save to a simple json file and be a fully usable app, that also tracks a few habits that help enforcing the good sleep schedule. it should also have a wizard command to set up the user's choices, which are limited to good practices according to the science of good sleep, and it should let the user set up his schedule, with also an option of waking up at the time when the sun rises in his area. the principle of the example is to use the minimum code possible for this app. it should have the default behaviour everywhere, without changing anything in the code that builds the flows, prompts, outputs, etc. the app should only set themes, and it should dynamically use one theme or another according to the user's sleeping schedule. showing a day theme when it's not time to sleep, and showing a sleep inspired night theme when it's close to time to sleep and during the night.

What follows from it:

1. **Minimum code.** Budget: **at most 550 lines of Rust** in `examples/lull/`, counting comments and blank lines, but not `cities.rs` (data) and not `#[cfg(test)]` modules. Report the count (`wc -l`, minus those parts) in the status notes. Prefer one clear line over a helper used once. No abstraction layers, no traits of the app's own (the one `impl` of a paloma trait, if any, is fine).
2. **Default behaviour everywhere.** The app calls only the *content* side of paloma's builders. See **The allowed API** below. It never sets options, outcomes, legends, layouts or per-prompt themes.
3. **Only themes.** The app's look is two themes made with `ColorfulTheme.restyle(..)` in `theme.rs`, plus `SimpleTheme` where the terminal has no Unicode. Nothing else in the app styles anything.
4. **The theme follows the schedule.** The night theme is shown from the start of the wind-down until the wake time; the day theme the rest of the time. It is chosen once, when the program starts.
5. **Choices limited to good practice.** Every wizard question offers only choices that sleep research supports (a fixed list, never free text for the schedule). The rules and their sources are in **Sleep rules** below; the copy says *why* in a few words, in descriptions of items and prompts.
6. **Sunrise wake.** The wake time can follow the local sunrise, computed offline from coordinates (no network), kept within a healthy window.
7. **Fully usable, saved to a simple JSON file.** Real clock, real file, every command works for daily use; setup can be run again and keeps the logged nights.
8. Paloma itself does not change. **No change to `src/`** is expected. If the app cannot work with the defaults, stop and record the gap in the status notes instead of changing paloma or working around it with options.

## The app

Name: **Lull**. Binary name in copy and hints: `lull`. Copy is lowercase, calm and short; it never scolds.

### Commands

| Command | What it does |
| --- | --- |
| `lull` | `today`, or `setup` when there is no schedule yet and stderr is a terminal. With no schedule and no terminal: the "no schedule" error (below), exit 1. |
| `lull setup` | The wizard (a `Flow`). Saves the schedule and habits; keeps logged nights and the parked note. |
| `lull today` | Tonight's plan and where you are in it right now. |
| `lull park` | Hyperfocus helper: write down where you stopped, so stopping is safe. Shown back by `lull log` in the morning. |
| `lull log` | Morning check-in for last night (a `Flow`): bedtime, wake time, how rested, which habits were kept. |
| `lull week` | The last 7 logged nights and how steady they were. |
| `lull forget` | Deletes the file after a `Confirm` (default no). |

Anything else: error `i do not know <arg>` with nested hint `lull setup, today, park, log, week or forget`, exit 2. There are no flags. Exit codes: 0, 1 (a message says why), 2 (unknown command); `Ctrl+C` exits 130 by paloma's default.

The "no schedule" error, used by `today`, `park`, `log` and `week` when the file does not exist: `paloma::error("no sleep schedule yet")` with nested `Message::hint("run: lull setup")`, exit 1. An unreadable file: `paloma::error(format!("{path} could not be read: {err}"))`, exit 1 (never overwrite it).

### Environment

- `LULL_FILE`: the file path. Default: `$HOME/.lull.json` (on Windows `%USERPROFILE%\.lull.json`). In copy the path is shown as given (`~` is not substituted).
- `LULL_NOW`: the current local time as `YYYY-MM-DDTHH:MM` (for previews and the golden tests). Default: `chrono::Local::now()`.
- The local time zone is chrono's `Local` (on Unix it honours `TZ`; the tests set `TZ=UTC`).

### Sleep rules

These are the app's whole "science". Copy may cite the short reason; the design doc lists the sources.

| Rule | Value in the app | Basis |
| --- | --- | --- |
| Sleep need | 7, 7.5, 8, 8.5 or 9 hours; default 8 | Adults need at least 7 hours (AASM/SRS consensus, Watson et al. 2015); 7–9 recommended (National Sleep Foundation, Hirshkowitz et al. 2015). |
| Same wake time every day | No weekend schedule exists | Regular timing keeps the body clock stable; irregular sleep is linked to worse sleep and health (e.g. Phillips et al. 2017). |
| Wind-down | 30, 45, 60 or 90 minutes; default 60 | A screen-free, work-free buffer before bed; evening light-emitting screens delay sleepiness and the body clock (Chang et al. 2015). |
| Parking the work | The `park` habit and command | Writing a to-do list before bed helped people fall asleep faster (Scullin et al. 2018). |
| Caffeine cut-off | 6, 8 or 10 hours before bed; default 8 | Caffeine 6 hours before bed still cut sleep by about an hour (Drake et al. 2013). |
| Sunrise wake window | Sunrise, clamped to 06:00–08:00 | Morning light anchors the body clock; the clamp keeps the time inside common healthy wake times when sunrise is very early or late. |
| Easing in | The whole night moves 15 minutes earlier every 3 days | Shifting a late schedule gradually rather than all at once is the usual advice for a delayed sleep phase. |

### Habits

Chosen in setup (`MultiSelect`, at least one), logged each morning. Stored by key.

| Key | Item text | Description (shown beside the item) | Checked by default |
| --- | --- | --- | --- |
| `park` | park your work | write where you stopped, then stop | yes |
| `screens` | screens off at wind-down | evening screens push sleep later | yes |
| `light` | daylight within an hour of waking | morning light sets the body clock | yes |
| `caffeine` | no caffeine after the cut-off | it lingers for hours | yes |
| `alcohol` | no alcohol in the 3 hours before bed | it breaks up the second half of the night | no |
| `naps` | naps under 20 minutes, before 15:00 | longer or later naps steal night sleep | no |

### Data file

`serde` derive, pretty-printed with `serde_json::to_string_pretty`, written with `std::fs::write`. Times and dates use chrono's serde (`"07:00:00"`, `"2026-10-11"`). The shape (field names exact):

```json
{
  "schedule": {
    "wake": { "at": "07:00:00" },
    "sleep_minutes": 480,
    "wind_down_minutes": 60,
    "caffeine_hours": 8,
    "ease": { "minutes": 120, "from": "2026-10-11" }
  },
  "habits": ["park", "screens", "light", "caffeine"],
  "nights": [
    { "date": "2026-10-10", "bed": "23:40:00", "up": "07:05:00", "rested": 3, "habits": ["park", "light"] }
  ],
  "parked": { "at": "2026-10-11T21:58:00", "note": "fix the flaky test in render.rs" }
}
```

- `wake` is an enum: `{ "at": "HH:MM:SS" }` or `{ "sunrise": { "place": "Barcelona", "lat": 41.39, "lon": 2.17 } }` (`#[serde(rename_all = "snake_case")]`, externally tagged, the serde default).
- `caffeine_hours` is `null` when the `caffeine` habit is not chosen. `ease` is `null` when not easing in. `parked` is `null` when nothing is parked.
- `nights[].date` is the date the night *started* (the evening). `rested` is 1–5. `nights` stays sorted by date; logging a night that exists replaces it.
- Habit keys and the wake enum are Rust enums with serde renames, so an unknown value is a read error, not a silent default.

## Files

- `examples/lull/main.rs`: read the clock and the file, choose the theme, dispatch the command, map errors to messages and exit codes.
- `examples/lull/commands.rs`: one function per command.
- `examples/lull/schedule.rs`: the data types (serde), load and save, and the pure time maths with its unit tests.
- `examples/lull/theme.rs`: `day()` and `night()`, plus the colour helper.
- `examples/lull/cities.rs`: a `const` table of about 80 cities `(name, country, lat, lon)`, 2 decimals, sorted by name, covering every continent and large time zones; data only.
- `Cargo.toml`:
  - Dev-dependencies: `chrono = { version = "0.4", default-features = false, features = ["clock", "serde", "std"] }`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `sunrise = "3"`.
  - An `[[example]] name = "lull"` with `path = "examples/lull/main.rs"`, `required-features = ["fuzzy-select"]` and `test = true`.
  - Commit the updated `Cargo.lock`.
- `deny.toml`: serde's derive pulls `unicode-ident`, licensed `(MIT OR Apache-2.0) AND Unicode-3.0`, and the allow list has only MIT and Apache-2.0. Add one narrow exception (do not widen the allow list):
  ```toml
  [[licenses.exceptions]]
  crate = "unicode-ident"
  allow = ["Unicode-3.0"]
  ```
- `tests/lull.rs` and `tests/fixtures/lull.json`, plus `tests/fixtures/lull-broken.json`: golden transcripts (see **Tests**).
- `docs/design/lull.md`: the two themes (colour table, glyphs), the theme switch rule, the sleep rules with full references, and the allowed-API rule.
- `docs/USAGE.md`: one paragraph after the Loam paragraph. `DEVELOPMENT.md`: mention `examples/lull/` and `tests/lull.rs` next to Loam. `ROADMAP.md`: move PAL-021 to Done when finished.

## The allowed API

The example may call only these paloma items. Everything else is out, so the app shows paloma's defaults.

- **Theme:**
  - `set_theme`, `ColorfulTheme`, `SimpleTheme` and `Theme::restyle`.
  - `Tokens::palette` / `Tokens::glyphs` and the `Palette` / `Glyphs` setters, only in `theme.rs`.
  - `terminal::capabilities(Stream::Stderr)` and its `is_tty` / `unicode` / `depth`.
- **Prompts:** `Select`, `MultiSelect`, `FuzzySelect`, `Input`, `Confirm`.
  - Their content builders: `new`, `prompt`, `items`, `items_checked`, `default`, `validate`, `description`, `placeholder`.
  - Their runners: `interact`, `interact_opt`.
  - `Item::new(..).description(..)`.
- **Flows:** `Flow::new`, `step`, `step_if`, `run`.
- **Output:** `paloma::section`, `paloma::info` / `success` / `warning` / `error`, `Message::hint` / `info` and `.nest(..)` / `.print()`, plus `KeyValue::new().row(..)` and `Table::new(..).rows(..)`, each with `.print()`.
- `paloma::Error` / `Result` and `Error::Cancelled` matching are not needed: cancel exits by default.

Not allowed:
- `set_options` and `Options`.
- `done`, `legend`, `choices_as_buttons`, `show_default`, `on_change` / `change_delay` / `pending`, `preset`.
- `with_theme`, `interact_on*` and stream overrides.
- `Flow::title` / `counter` / `esc_back` / `keep_last` / `done` / `step_done` / `theme`, and `flow::back_to`.
- Tasks, `Panel`, links and gradients: they are not needed by this app, and adding them for show breaks rule 1.
- Raw escape codes, and `console::Style` or `style(` outside `theme.rs`.

## Implementation steps

### 1. Dependencies and skeleton

Add the dev-dependencies, the `[[example]]`, the `deny.toml` exception. `cargo check --example lull --features fuzzy-select` on an empty `main`.

### 2. `schedule.rs`: data, file, maths

Types, all `#[derive(Serialize, Deserialize, Clone, Debug)]` (add `PartialEq` where tests need it):

```rust
pub struct Data { pub schedule: Schedule, pub habits: Vec<Habit>, pub nights: Vec<Night>, pub parked: Option<Parked> }
pub struct Schedule { pub wake: Wake, pub sleep_minutes: u32, pub wind_down_minutes: u32, pub caffeine_hours: Option<u32>, pub ease: Option<Ease> }
pub enum Wake { At(NaiveTime), Sunrise(Place) }
pub struct Place { pub place: String, pub lat: f64, pub lon: f64 }
pub struct Ease { pub minutes: u32, pub from: NaiveDate }
pub enum Habit { Park, Screens, Light, Caffeine, Alcohol, Naps }   // serde rename_all = "snake_case"
pub struct Night { pub date: NaiveDate, pub bed: NaiveTime, pub up: NaiveTime, pub rested: u8, pub habits: Vec<Habit> }
pub struct Parked { pub at: NaiveDateTime, pub note: String }
```

`Habit` has `const ALL: [Habit; 6]` and `fn text(self) -> &'static str` / `fn why(self) -> &'static str` with the table above.

File:
- `pub fn path() -> PathBuf`: `LULL_FILE`, else the home directory + `.lull.json`.
- `pub fn load(path) -> Result<Option<Data>, String>`: a missing file → `Ok(None)`, any other error → `Err(text)`.
- `pub fn save(path, &Data) -> Result<(), String>`.

The maths are pure functions. Pass the time zone in as `&impl TimeZone` so tests use `FixedOffset`.
- **`wake_on(schedule, date, tz) -> NaiveTime`**: the wake time on the morning of `date`, before easing.
  - `At(t)`: `t`.
  - `Sunrise(p)`: `SolarDay::new(Coordinates::new(p.lat, p.lon)?, date).event_time(SolarEvent::Sunrise)` in `tz`, clamped to 06:00..=08:00. When there is no sunrise (polar day or night) or the coordinates are invalid: 08:00.
- **`ease_minutes(schedule, night_date) -> u32`**: `ease.minutes.saturating_sub(15 * (days since ease.from, at least 0) / 3)`, with integer division of the days: 0–2 days → full, 3–5 → minus 15, ….
- **`plan(schedule, night_date, tz) -> Plan`**: `Plan { caffeine: Option<NaiveDateTime>, wind_down, bed, wake: NaiveDateTime, ease: u32 }` for the night that *starts* on `night_date`.
  - `wake = (night_date + 1 day) at wake_on(.., night_date + 1, ..) + ease`.
  - `bed = wake - sleep_minutes`, `wind_down = bed - wind_down_minutes`, `caffeine = bed - caffeine_hours`.
  - All are `NaiveDateTime`, so midnight needs no special case.
- **`phase(schedule, now, tz) -> (Phase, Plan)`**, with `enum Phase { Day, WindDown, Night }`. Look at the nights starting on `now.date() - 1` and `now.date()`.
  - `now` in `[wind_down, bed)` of either → `WindDown`; in `[bed, wake)` → `Night`.
  - Otherwise `Day`, with the plan of the next night that has not ended: the night starting today if `now` is before its wake, else tomorrow's.
  - The returned plan is the night `now` belongs to, or the next one.
- **`night_date(now) -> NaiveDate`**: the date the night being logged started: `(now - 12 hours).date()`. At 07:30 on the 12th it is the 11th; at 23:00 on the 11th it is also the 11th.
- **`minutes_between(a: NaiveTime, b: NaiveTime) -> i64`**: `b - a` in minutes, taken in (-720, 720], across midnight (23:30 → 00:30 is +60).
- **`spread(times: &[NaiveTime]) -> u32`**: the latest minus the earliest bedtime, in minutes, across midnight. Measure each time as minutes from 18:00, so 23:30 and 00:30 are 60 apart.
- **`ease_for(target_bed: NaiveTime, usual: NaiveTime) -> Option<u32>`**: `minutes_between(target_bed, usual)` rounded down to a multiple of 15, when it is more than 30; else `None`.
- **`hm(minutes) -> String`**: `7 h 25 min`, `45 min`, `2 h`.

### 3. `theme.rs`: two themes

Both are `ColorfulTheme.restyle(|t| t.palette(|p| …).glyphs(|g| …))`.
- They set only palette roles and the glyphs below.
- The colour helper is `fn c(depth: ColorDepth, i256: u8, basic: Style) -> Style`: `Style::new().color256(i256)` at `Ansi256` and `TrueColor`, `basic` at `Basic`.
- `pub fn day(depth) -> impl Theme` and `pub fn night(depth) -> impl Theme`.

| Role | Day "daylight" (256 / basic) | Night "moonlight" (256 / basic) |
| --- | --- | --- |
| `text` | unset (terminal default) | 146 / unset |
| `muted` | unset | 103 / unset |
| `subtle` | 244 / bright black | 60 / bright black |
| `faint` | 244 / bright black | 238 / bright black |
| `accent` | 214 / yellow | 104 / blue |
| `accent_strong` | 208 bold / yellow bold | 147 (not bold) / bright blue |
| `success` | 71 / green | 108 / green |
| `info` | 74 / cyan | 110 / cyan |
| `warning` | 178 / yellow | 180 / yellow |
| `error` | 167 / red | 174 / bright red |

| Glyph | Day | Night |
| --- | --- | --- |
| `prompt` | `☼` | `☾` |
| `answered` | ColorfulTheme's | `⋆` |
| `section` | `☼` | `✧` |

The night theme is deliberately low-contrast and never bold: it is read in a dark room, minutes before sleep. All glyphs are one column wide in `console::measure_text_width`, so assert that in a unit test.

`main.rs` picks the theme:
- With no Unicode, `set_theme(SimpleTheme)`.
- Otherwise `night(depth)` when the phase is `WindDown` or `Night`, and `day(depth)` for `Day`.
- When there is no schedule, the default schedule decides the phase: `At(07:00)`, 480 minutes, 60 minutes wind-down. This makes setup at night use the night theme.
- `NO_COLOR` needs no code (console handles it).

### 4. `main.rs`

1. `now` (`LULL_NOW` or `Local::now().naive_local()`; a malformed `LULL_NOW` is error exit 1), `path`, and `load`.
2. The theme, from step 3.
3. The command from `env::args().nth(1)`, with the dispatch from the Commands table.
4. Each command returns `Result<(), Box<dyn std::error::Error>>`. On error, `paloma::error(err.to_string()).print()` and exit 1.

### 5. `commands.rs`

**`setup(data: Option<Data>, path, now)`** is one `Flow::new(state)`. The state starts from the existing data or the defaults (fixed 07:00, 8 h, 60 min, the four default habits, caffeine 8 h). The steps, in order:

| Step | Prompt | Items / rules |
| --- | --- | --- |
| `wake` | `Select` "how do you want to wake up?" | `Item::new("at a set time").description("the same time every day, weekends too")`, `Item::new("with the sunrise").description("between 06:00 and 08:00, following the sun where you live")` |
| `time` (if set time) | `Select` "wake up at" | 05:30 to 09:00 every 30 minutes; default the current one or 07:00 |
| `place` (if sunrise) | `FuzzySelect` "where do you live?", `placeholder("search cities")` | the cities as `"Barcelona, Spain"`, then the last item `"somewhere else"` with description `"enter coordinates"` |
| `coordinates` (if somewhere else) | `Input::<String>` "latitude, longitude" with `description("like 41.39, 2.17")` | validate: two numbers that `Coordinates::new` accepts, else `"two numbers, like 41.39, 2.17"` |
| `sleep` | `Select` "how much sleep do you need?" with `description("adults need 7 to 9 hours")` | `7 h`, `7 h 30 min`, `8 h`, `8 h 30 min`, `9 h`; default 8 h. |
| `wind down` | `Select` "wind down for" with `description("a hard stop for screens and work")` | 30, 45, 60, 90 min, default 60. Item 60 has description `"good for evening hyperfocus"`. |
| `usual` | `Select` "when do you fall asleep these days?" | "around the time above" (none), then 23:00 to 03:00 every 30 minutes, then "later than 03:00" (03:30). Default: the first item. |
| `habits` | `MultiSelect` "which habits will you keep?" | `items_checked` with `Item::new(text).description(why)`, checked as in the data; validate at least one: `"pick at least one habit"` |
| `caffeine` (if the caffeine habit) | `Select` "last caffeine" | `6 hours before bed`, `8 hours before bed`, `10 hours before bed`; default 8 |

- **The `sleep` step's description:** when the wake follows the sunrise, append the wake time it gives tomorrow, e.g. `adults need 7 to 9 hours · tomorrow you wake at 07:59`. This is the only place the sunrise shows before saving; one `format!`.
- **After `run()`:**
  - Compute `ease` from `usual` with `ease_for` and today as `from`, then save.
  - Print `paloma::section("your schedule")` and a `KeyValue` with these rows: `wake` (`07:00`, or `with the sunrise in Barcelona, 06:00-08:00`), `sleep` (`8 h`), `wind down` (`60 min`), `caffeine` (`until 15:00`, or no row), `habits` (texts joined with `, `).
  - Then `paloma::success("saved")` with nested `Message::hint(format!("{path}"))`.
  - When easing, also `paloma::info(format!("starting at {bed}, the whole night moves 15 min earlier every 3 days"))`.

**`today(data, now)`:**
- The `section("tonight")` and a `KeyValue` of the plan: `caffeine until`, `wind down`, `bed`, `wake`. Every time is `HH:MM`. `wake` gets ` (sunrise)` when it follows the sunrise.
- Then one status message, by phase:
  - `Day`: `info(format!("wind-down in {}", hm(..)))`.
  - `WindDown`: `warning(format!("wind-down started {} ago", hm(..)))` with nested hint `park your work: lull park` when the `park` habit is chosen, else hint `screens off`.
  - `Night`: `warning(format!("{} past bedtime", hm(..)))` with nested hint `it will all still be there tomorrow`. Use `info("time to sleep")` instead within the first 10 minutes after bedtime.
- Then, if easing (`ease > 0`): `Message::hint(format!("easing in: {} later than your goal, 15 min earlier on {date}", hm(ease)))`. The date is the next day the ease steps down, written `Oct 14` (`%b %-d`).
- Then, after wake and before noon with no night logged for `night_date(now)`: `Message::hint("good morning: lull log")`.

**`park(data, path, now)`:**
- `Input::<String>` "where did you stop?" with `placeholder("the next thing to do, in a few words")`, and validate not blank: `"a few words is enough"`.
- Save `Parked { at: now, note }`, then print `success("parked")` with nested hint `it will be here in the morning`.
- A second `park` the same evening replaces the note.

**`log(data, path, now)`:**
- A `Flow` with the plan of the night starting on `night_date(now)`. Its steps:
  - `bed`: `Input::<String>` "went to bed at", default the plan's bed `HH:MM`, validated as `HH:MM`: `"a time like 23:30"`.
  - `up`: the same, "got up at", default the plan's wake.
  - `rested`: `Select` "how rested do you feel?": `drained`, `tired`, `okay`, `rested`, `great`; default `okay`.
  - `habits` (if any are chosen): `MultiSelect` "which habits did you keep?" over the chosen habits, none checked.
- Then save the night, and:
  - if something was parked, print `info("you parked:")` with nested `Message::info(note)`, then clear it;
  - print `success(format!("logged {}", hm(slept)))`;
  - when `minutes_between(plan bed, bed) > 30`, print `warning(format!("{} later than planned", hm(..)))` with nested hint `start the wind-down on time tonight`.
- `slept` is `minutes_between(bed, up)`, taken as positive across midnight, so `(up - bed)` wraps by 24 h.

**`week(data)`:**
- No nights: `info("no nights logged yet")` with nested hint `lull log, in the morning`.
- Otherwise the last 7 nights, oldest first, as a `Table` with these columns:
  - `night` (`Sat 10`, `%a %-d`),
  - `bed`, `up`,
  - `slept` (`hm`),
  - `rested` (the word),
  - `habits` (`2/4`, kept / chosen).
- Then a `KeyValue`:
  - `average sleep` (`hm`),
  - `bedtime spread` (`hm(spread)`),
  - `on time` (`5 of 7`: bed within 30 minutes of that night's plan, recomputed with the current schedule),
  - `habits kept` (`68%`).
- Then one message:
  - spread up to 60 → `success("a steady week")`;
  - otherwise `warning(format!("your bedtime moved by {} this week", hm(spread)))` with nested hint `the wind-down is the anchor: lull today`.

**`forget(path)`:** `Confirm` "delete {path}?" with `default(false)`; yes → `fs::remove_file` and `success("forgotten")`; no → `info("kept")`. A missing file → the "no schedule" error.

### 6. Tests, docs, roadmap

See below. Then `docs/design/lull.md`, `USAGE.md`, `DEVELOPMENT.md`, `ROADMAP.md`.

## Tests and acceptance criteria

**Existing output.** Every existing snapshot (`theme-*`, `ultraviolet-*`, `loam-*`) stays byte-identical, and `src/` is unchanged.

**Unit tests** (`schedule.rs` and `theme.rs`, run by `cargo test` through `test = true`):
- `the_night_crosses_midnight`: `At(07:00)`, 8 h, 60 min, caffeine 8 h, night of 2026-10-11. The plan is wake 10-12 07:00, bed 10-11 23:00, wind down 22:00 and caffeine 15:00.
- `phases_follow_the_plan`: with that schedule, 2026-10-11 at 14:00 is `Day`, 22:30 is `WindDown` and 23:30 is `Night`. 2026-10-12 at 03:00 is `Night` with the plan of the 11th, and 2026-10-12 at 07:00 is `Day`.
- `sunrise_is_clamped`:
  - Barcelona (41.39, 2.17) on 2026-10-12, with `FixedOffset` +2 h, gives the computed sunrise, inside the window.
  - Helsinki (60.17, 24.94) on 2026-06-21, +3 h, gives 06:00.
  - Reykjavík (64.15, -21.94) on 2026-12-21, +0, gives 08:00.
  - Tromsø (69.65, 18.96) on 2026-12-21 (no sunrise) gives 08:00.
- `easing_steps_every_three_days`: 120 minutes from 2026-10-11 gives 120 on the 11th and the 13th, 105 on the 14th, 15 on 2026-11-03, and 0 from 2026-11-04 on.
- `ease_needs_more_than_half_an_hour`: target 23:00 with usual 23:30 gives `None`, usual 01:10 gives `Some(120)`, and usual 22:00 gives `None`.
- `spread_and_minutes_cross_midnight`: `spread([23:30, 00:30, 23:45])` is 60, and `minutes_between(23:30, 00:30)` is 60.
- `night_date_is_the_evening`.
- `hm_reads_naturally`.
- `the_data_file_round_trips`: the fixture parses, and saving and loading again gives equal data.
- `glyphs_are_one_column` in `theme.rs`.

**Golden transcripts** (`tests/lull.rs`, Unix and `fuzzy-select` only, reusing `tests/common` and the build-once pattern of `tests/loam.rs`):
- **Environment:** every run sets `TZ=UTC` and `LULL_FILE=tests/fixtures/lull.json` (read-only: these commands never write). `LULL_NOW` varies.
- **The fixture:**
  - Wake `At(07:00)`, 8 h, 60 min wind-down, caffeine 8 h, the `park`, `screens`, `light` and `caffeine` habits, no ease.
  - 9 nights ending on 2026-10-10, two of them with a bedtime after midnight, and a parked note.
- **Snapshots:**
  - `lull-day.txt`: 256 colours (force the colour depth the way `tests/loam.rs` does), with `today` at `2026-10-11T14:00`, plus `week`, `nope` and `today` with `LULL_FILE` pointing at a missing file.
  - `lull-wind-down.txt`: 256 colours, `today` at `2026-10-11T22:20`.
  - `lull-night.txt`: 256 colours, `today` at `2026-10-12T00:40` and `week` at the same time (the night theme on a table).
  - `lull-morning.txt`: 256 colours, `today` at `2026-10-11T07:30` (the night of the 10th is logged: no hint) and at `2026-10-12T07:30` (the night of the 11th is not: the `good morning: lull log` hint shows).
  - `lull-16.txt`: basic colours, `today` at 14:00 and at 00:40.
  - `lull-no-color.txt`: `NO_COLOR=1`, `today` at 00:40.
  - `lull-ascii.txt`: a non-Unicode terminal (`SimpleTheme`), `today` and `week`.
  - `lull-broken.txt`: `LULL_FILE=tests/fixtures/lull-broken.json`. `today` exits 1 with the read error.

**Pty checks** (Python `pty` + `pyte`, as in DEVELOPMENT.md; `TZ=UTC`; `LULL_FILE` in a temp dir). Record the final screens in the status notes.
- `setup` with no file:
  - With the sunrise: type `barc`, Enter, and check the `sleep` step shows tomorrow's wake.
  - Esc back to `wake`, then switch to `at a set time` and check the place step is skipped.
  - Choose `00:30` as usual and the four default habits, and check the summary and the easing info.
  - `cat` the file to confirm it matches the data shape.
  - `somewhere else` with `91, 0`: the validation message shows; `41.39, 2.17` passes.
- `today` at `LULL_NOW=…T22:20`: the night theme (check cell colours in pyte: the accent is 256-index 104). At `…T14:00`: the day theme (accent 214).
- `park`, then `log` at the next morning: defaults are the plan's times, the parked note shows, and the file has the night and `"parked": null`.
- `forget` then No, then `forget` then Yes.
- `^C` in `setup`: exit 130, nothing saved.

**Code rules,** checked with grep and reported:
- Outside `theme.rs`, `examples/lull/` has no `\x1b`, `Style`, `style(`, `console::` or colour names.
- None of the forbidden builders from **The allowed API** appear.
- The 550-line budget is met.

**Full check set** from DEVELOPMENT.md:
- `cargo test --locked` with all, default and no features. The example and its test need `fuzzy-select`, so they are skipped without it.
- `cargo lint` and `cargo clippy --locked --all-targets --no-default-features -- -D warnings`.
- `cargo format-check` and `cargo check --examples --all-features`.
- `RUSTDOCFLAGS=-Dwarnings cargo doc --locked --all-features --no-deps --document-private-items`.
- `cargo +1.85.0 check --locked --lib --all-features`.

## Out of scope

- **Reminders or notifications at wind-down time** (a daemon, a shell prompt hook): the app is run by hand. Worth a later item if wanted.
- **Spinners and progress bars:** nothing in the app takes time.
- **Weekend or per-day schedules,** naps as entries, sleep stages and wearables. These are deliberately out, because the choices are limited to good practice.
- **Changing paloma.** Any gap goes in the status notes.
- **Time zone travel:** the schedule follows the clock of wherever the computer is.

## Status notes

- 2026-10-11: Plan written. Executor: Sonnet 5.5, medium effort. Checked beforehand: `sunrise` 3.0.0 (MIT; `SolarDay::new(Coordinates::new(lat, lon)?, date).event_time(SolarEvent::Sunrise) -> Option<DateTime<Utc>>`, `None` in polar night) gives 05:58 UTC for Barcelona on 2026-10-11, which is 07:58 local; it needs chrono 0.4 and edition 2024 (Rust 1.85, so the MSRV is fine, and the example is outside the lib-only MSRV check anyway). `unicode-ident` (pulled by serde derive) needs the `deny.toml` exception above.
