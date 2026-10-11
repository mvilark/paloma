//! One function per command. Everything shown goes through paloma: prompts, flows,
//! messages, blocks and tasks.
use std::{thread, time::Duration};

use paloma::{
    flow,
    terminal::Stream,
    text::{gradient, link},
    BasicHistory, Completion, Confirm, Done, Editor, Flow, FuzzySelect, History, Input, Item,
    KeyValue, Message, MultiSelect, Panel, PanelRow, Password, Result, Select, Sort, Status, Table,
};

use crate::{
    data::{
        is_taken, seed_index, Bed, BEDS, COMPANIONS, CROPS, FIRST_FROST_DAYS, JOURNAL, SEEDS,
        STATUS_JSON, WEEK, ZONES,
    },
    theme, Args,
};

/// Fake work: waits `ms` when there is motion. Without motion (CI, pipes, `--yes` in
/// the transcripts) nothing waits, so every duration reads `0.0s`.
fn pause(args: &Args, ms: u64) {
    if args.motion {
        thread::sleep(Duration::from_millis(ms));
    }
}

/// `1 seed`, `3 seeds`.
fn count(n: usize, one: &str) -> String {
    format!("{n} {one}{}", if n == 1 { "" } else { "s" })
}

// ---------------------------------------------------------------------------------
// Progress seams: three places where a progress bar (PAL-011) is the right component.
// Each is one small function that takes the units and runs a task with one step per
// unit. When progress bars land, only these functions change.
// ---------------------------------------------------------------------------------

/// PAL-011: a progress bar "sowing" over `seeds.len()`, its label naming the seed being
/// sown, instead of this task.
fn sow(args: &Args, seeds: &[&str]) -> Result {
    let task = paloma::task("sowing...").start()?;
    for seed in seeds {
        pause(args, 450);
        task.step(format!("sowed {seed}"))?;
    }
    task.success(format!("sowed {}", count(seeds.len(), "seed")))?;
    Ok(())
}

/// PAL-011: a progress bar per bed, or one over the total litres, instead of this task.
fn water_beds(args: &Args, beds: &[&Bed]) -> Result {
    let task = paloma::task("watering...").start()?;
    let mut litres = 0;
    for bed in beds {
        pause(args, 500);
        litres += u32::from(bed.litres);
        task.step(format!("watered {} ({} litres)", bed.name, bed.litres))?;
    }
    task.success(format!(
        "watered {}, {litres} litres",
        count(beds.len(), "bed")
    ))?;
    Ok(())
}

/// PAL-011: a progress bar over the 40 seeds, eased, instead of this task (one step per
/// ten seeds).
fn fetch_catalogue(args: &Args) -> Result {
    let task = paloma::task("fetching the seed catalogue...").start()?;
    for from in (1..=SEEDS.len()).step_by(10) {
        pause(args, 300);
        task.step(format!("fetched seeds {from}-{}", from + 9))?;
    }
    task.success(format!("fetched {} seeds", SEEDS.len()))?;
    Ok(())
}

// ---------------------------------------------------------------------------------

/// The wordmark: `loam` in the theme's gradient, as a section title so it has air around
/// it, then a one-line tagline.
pub fn wordmark(args: &Args) -> Result {
    let (from, to) = theme::wordmark(args.night);
    paloma::section(gradient("loam", from, to).to_string()).print()?;
    paloma::description("a garden journal for slow afternoons").print()
}

/// Why a garden name is refused: its shape, or the neighbours already use it.
fn name_problem(name: &str) -> Option<Message<'static>> {
    let name = name.trim();
    let letters = name.chars().filter(|c| c.is_alphanumeric()).count();
    if !(2..=24).contains(&name.chars().count())
        || letters == 0
        || !name.chars().all(|c| c.is_alphanumeric() || c == ' ')
    {
        return Some(
            Message::error("a name takes 2 to 24 letters, digits or spaces")
                .nest(Message::hint("like the back plot")),
        );
    }
    is_taken(name).then(|| {
        Message::warning(format!("{name} is taken"))
            .nest(Message::hint("the neighbours use it already"))
    })
}

/// The rules a garden password must meet, as a message that lists those still missing.
fn password_rules(password: &str) -> Option<Message<'static>> {
    let missing: Vec<&str> = [
        (password.chars().count() < 8, "at least 8 characters"),
        (!password.chars().any(|c| c.is_ascii_digit()), "one digit"),
    ]
    .into_iter()
    .filter_map(|(fails, rule)| fails.then_some(rule))
    .collect();
    (!missing.is_empty()).then(|| {
        missing.into_iter().fold(
            Message::warning("not strong enough yet"),
            |message, rule| message.nest(Message::hint(rule)),
        )
    })
}

/// What the new-garden flow asks for. Earlier answers stay here when the user goes
/// back with Esc, so each step shows them again.
struct Garden {
    name: String,
    zone: usize,
    beds: u8,
    sun: bool,
    shade: usize,
    password: String,
    /// The soil test has already warned about too many beds.
    warned: bool,
}

const SHADE: [&str; 3] = ["about 2 hours", "about 4 hours", "about 6 hours"];

pub fn init(args: &Args) -> Result<i32> {
    let zone = match args.zone.as_deref() {
        None => args.yes.then_some(0),
        Some(wanted) => match ZONES
            .iter()
            .position(|z| z.name.eq_ignore_ascii_case(wanted))
        {
            Some(i) if ZONES[i].open => Some(i),
            Some(_) => {
                paloma::warning(format!("{wanted} is not open yet"))
                    .nest(Message::hint("loam init --zone temperate"))
                    .print()?;
                return Ok(1);
            }
            None => {
                paloma::error(format!("there is no zone called {wanted}"))
                    .nest(Message::hint(
                        "try temperate, cool, mediterranean, tropical or arid",
                    ))
                    .print()?;
                return Ok(1);
            }
        },
    };
    let name = match &args.name {
        Some(name) => {
            if let Some(problem) = name_problem(name) {
                problem.print()?;
                return Ok(1);
            }
            Some(name.trim().to_string())
        }
        None => args.yes.then(|| "the back plot".to_string()),
    };
    let motion = args.motion;

    let garden = Flow::new(Garden {
        name: String::new(),
        zone: 0,
        beds: 4,
        sun: true,
        shade: 1,
        password: String::new(),
        warned: false,
    })
    .title("new garden")
    .keep_last(3)
    .done(Done::Collapse)
    .step("name", |g| {
        g.name = Input::<String>::new()
            .prompt("name your garden")
            .placeholder("the back plot")
            .description("what do you call this patch of earth?")
            .prefill(g.name.clone())
            .validate(|name: &String| name_problem(name).map_or(Ok(()), Err))
            // A fake "is the name taken" call, once typing pauses.
            .change_delay(400)
            .pending("asking the neighbours...")
            .on_change(move |text: &str| {
                if motion {
                    thread::sleep(Duration::from_millis(600));
                }
                if text.trim().is_empty() {
                    None
                } else if is_taken(text) {
                    name_problem(text)
                } else {
                    Some(Message::success("nobody else uses that name"))
                }
            })
            .preset(name.clone())
            .interact()?;
        Ok(())
    })
    .step("zone", |g| {
        g.zone = Select::new()
            .prompt("pick your climate zone")
            .items(
                ZONES
                    .iter()
                    .map(|z| Item::new(z.name).description(z.about).disabled(!z.open)),
            )
            .default(g.zone)
            .legend(true)
            .preset(zone)
            .interact()?;
        Ok(())
    })
    .step("beds", |g| {
        g.beds = Input::<u8>::new()
            .prompt("how many beds?")
            .default(g.beds)
            .validate(|n: &u8| {
                if (1..=12).contains(n) {
                    Ok(())
                } else {
                    Err("a garden has 1 to 12 beds")
                }
            })
            .interact()?;
        Ok(())
    })
    .step("sun", |g| {
        g.sun = Confirm::new()
            .prompt("does it get full sun?")
            .default(g.sun)
            .interact()?;
        Ok(())
    })
    .step_if(
        |g| !g.sun,
        "shade",
        |g| {
            g.shade = Select::new()
                .prompt("how many hours of sun?")
                .items(SHADE)
                .default(g.shade)
                .interact()?;
            Ok(())
        },
    )
    // This step asks nothing, so `Esc` at the next one skips it.
    .step("soil test", |g| {
        let task = paloma::task("testing the soil...").start()?;
        for result in ["pH 6.4", "nitrogen low", "drainage good"] {
            pause(args, 350);
            task.step(result)?;
        }
        pause(args, 350);
        if g.beds > 8 && !g.warned {
            g.warned = true;
            task.fail("the soil is worn out")?;
            return flow::back_to(
                "beds",
                Message::warning("the soil can feed 8 beds this year")
                    .nest(Message::hint("try fewer beds, or add compost first")),
            );
        }
        task.success("the soil is ready")?;
        Ok(())
    })
    .step("companion", |g| {
        g.password = Password::new()
            .prompt("a garden password, for sharing")
            .mask(true)
            .confirmation("once more", "those two do not match")
            .on_change(password_rules)
            .validate(|password: &String| password_rules(password).map_or(Ok(()), Err))
            .preset(args.yes.then(|| "sharing1".to_string()))
            .interact()?;
        Ok(())
    })
    .run()?;

    paloma::section("your garden").print()?;
    let sun = if garden.sun {
        "full sun".to_string()
    } else {
        SHADE[garden.shade].to_string()
    };
    KeyValue::new()
        .row("name", garden.name.as_str())
        .row("zone", ZONES[garden.zone].name)
        .row("beds", garden.beds.to_string())
        .row("sun", sun)
        .row("sharing", "kept behind a password")
        .print()?;
    paloma::success(format!("{} is ready", garden.name))
        .nest(Message::hint("next: loam plant"))
        .print()?;
    Ok(0)
}

pub fn plant(args: &Args) -> Result<i32> {
    fetch_catalogue(args)?;
    let basil = seed_index("basil").unwrap_or(0);
    let seed = FuzzySelect::new()
        .prompt("which seed?")
        .items(
            SEEDS
                .iter()
                .map(|s| Item::new(s.name).description(format!("sow in {}", s.season))),
        )
        .placeholder("search seeds...")
        .highlight_matches(true)
        .default(basil)
        .interact()?;
    let seed = &SEEDS[seed];

    let beside = MultiSelect::new()
        .prompt(format!("anything beside the {}?", seed.name))
        .items_checked(COMPANIONS.iter().map(|c| {
            (
                Item::new(c.name).description(c.why).disabled(!c.in_season),
                c.checked,
            )
        }))
        .validate(|picked: &[usize]| {
            if picked.is_empty() {
                Err("choose at least one companion")
            } else {
                Ok(())
            }
        })
        .legend(true)
        .interact()?;
    let mut sowing = vec![seed.name];
    sowing.extend(beside.iter().map(|&i| COMPANIONS[i].name));

    // The classic `(y/n)` layout, beside the buttons the rest of the app uses.
    let go = Confirm::new()
        .prompt("sow them in bed 2?")
        .choices_as_buttons(false)
        .default(true)
        .interact_opt()?;
    if go != Some(true) {
        paloma::info("nothing was sown").print()?;
        return Ok(0);
    }

    sow(args, &sowing)?;
    Panel::new("sown")
        .rows(sowing.iter().map(|name| {
            let days = seed_index(name).map_or(seed.sprouts_in(), |i| SEEDS[i].sprouts_in());
            PanelRow::new(format!("{name} in bed 2"))
                .status(Status::Success)
                .trailing(format!("sprouts in {days}"))
        }))
        .print()?;
    Ok(0)
}

pub fn water(args: &Args) -> Result<i32> {
    Table::new(["bed", "plants", "last watered", "moisture"])
        .rows(BEDS.iter().map(|b| {
            [
                b.name.to_string(),
                b.plants.to_string(),
                b.watered.to_string(),
                format!("{}%", b.moisture),
            ]
        }))
        .print()?;
    let picked = MultiSelect::new()
        .prompt("which beds?")
        .description("the dry ones are already checked")
        .items_checked(BEDS.iter().map(|b| (b.name, b.is_dry())))
        .legend(true)
        // A question that is only a means: it leaves nothing behind.
        .done(Done::Erase)
        .interact()?;
    if picked.is_empty() {
        paloma::info("every bed is damp enough").print()?;
        return Ok(0);
    }
    let beds: Vec<&Bed> = picked.iter().map(|&i| &BEDS[i]).collect();
    water_beds(args, &beds)?;
    paloma::info("the rain barrel is half full").print()?;
    Ok(0)
}

pub fn harvest(args: &Args) -> Result<i32> {
    let order = Sort::new()
        .prompt("what to pick first")
        .items(CROPS.iter().map(|c| c.name))
        .legend(true)
        .interact()?;

    let task = paloma::task("picking...").start()?;
    let mut picked = Vec::new();
    let mut overripe = None;
    for &i in &order {
        let crop = &CROPS[i];
        pause(args, 450);
        if crop.overripe {
            // A prompt pauses the spinner and brings it back below the answer.
            let anyway = Confirm::new()
                .prompt(format!("the {} is huge. pick it anyway?", crop.name))
                .default(true)
                .interact()?;
            overripe = Some((crop.name, anyway));
            if !anyway {
                task.step(format!("left the {} on the vine", crop.name))?;
                continue;
            }
        }
        picked.push(crop);
        task.step(format!("picked {}", crop.name))?;
    }
    let crops = count(picked.len(), "crop");
    match overripe {
        Some((name, true)) => task.warning(format!("picked {crops}, the {name} was overripe"))?,
        Some((name, false)) => task.warning(format!("picked {crops}, the {name} stays on"))?,
        None => task.success(format!("picked {crops}"))?,
    };

    let total: f32 = picked.iter().map(|c| c.kilos).sum();
    KeyValue::new()
        .rows(
            picked
                .iter()
                .map(|c| (c.name, format!("{:.1} kg", c.kilos))),
        )
        .row("in all", format!("{total:.1} kg"))
        .print()?;
    Ok(0)
}

/// Completes the last word of a note to a plant name, when only one plant fits.
struct Plants;

impl Completion for Plants {
    fn get(&self, input: &str) -> Option<String> {
        let (head, word) = input.rsplit_once(' ').map_or(("", input), |(h, w)| (h, w));
        if word.is_empty() {
            return None;
        }
        let mut fits = SEEDS.iter().filter(|s| s.name.starts_with(word));
        let (first, second) = (fits.next()?, fits.next());
        if second.is_some() {
            return None;
        }
        let space = if head.is_empty() && !input.contains(' ') {
            ""
        } else {
            " "
        };
        Some(format!("{head}{space}{}", first.name))
    }
}

pub fn journal(args: &Args) -> Result<i32> {
    if args.long {
        return long_note(args);
    }
    // Old notes come back with the up arrow; the history lives in memory only.
    let mut history = BasicHistory::new().max_entries(20).no_duplicates(true);
    for note in JOURNAL {
        history.write(&note.to_string());
    }
    Input::<String>::new()
        .prompt("write a note")
        .placeholder("what did the garden do today?")
        .description("up brings back old notes, tab completes a plant")
        .history(&mut history)
        .completion(&Plants)
        .default("watered the herb spiral and checked the seedlings".to_string())
        .show_default(false)
        // The note stays as typed.
        .done(Done::Keep)
        .interact()?;
    paloma::success("noted")
        .nest(Message::hint("loam forgets it when it closes"))
        .print()?;
    Ok(0)
}

/// `journal --long`: the note is written in the editor and printed back.
fn long_note(args: &Args) -> Result<i32> {
    // An unattended run has nobody to write the note.
    if args.yes {
        paloma::info("no editor opens in an unattended run").print()?;
        return Ok(0);
    }
    match Editor::new()
        .extension(".md")
        .edit("what did the garden do today?\n")?
    {
        Some(text) => {
            paloma::success("note written").print()?;
            println!("{text}");
        }
        None => paloma::warning("nothing was saved")
            .nest(Message::hint("save the file before you close the editor"))
            .print()?,
    }
    Ok(0)
}

pub fn status(args: &Args) -> Result<i32> {
    if args.json {
        println!("{STATUS_JSON}");
        return Ok(0);
    }
    let task = paloma::task("checking the sky...").start()?;
    pause(args, 700);
    task.clear()?;

    KeyValue::new()
        .row("garden", "the back plot")
        .row("zone", ZONES[0].name)
        .row("beds", format!("{}, 14 plants", count(BEDS.len(), "bed")))
        .row("last watered", "yesterday")
        .row("next harvest", "tomatoes, in 5 days")
        .print()?;
    paloma::warning(WEEK[0].outlook)
        .nest(Message::hint("cover the seedlings in bed 3"))
        .print()?;
    paloma::error("the bean teepee has not been watered in 9 days")
        .nest(Message::hint("run: loam water"))
        .print()?;
    Ok(0)
}

pub fn almanac(args: &Args) -> Result<i32> {
    wordmark(args)?;
    let rain = WEEK.iter().find(|d| d.outlook.contains("rain"));
    let mut this_week = Panel::new("this week").row(
        PanelRow::new(WEEK[0].outlook)
            .status(Status::Warning)
            .trailing(format!("low of {}", WEEK[0].low)),
    );
    if let Some(day) = rain {
        this_week = this_week.row(
            PanelRow::new("rain on the way")
                .status(Status::Info)
                .trailing(day.name),
        );
    }
    this_week
        .row(
            PanelRow::new("first frost of autumn")
                .status(Status::Info)
                .trailing(format!("{FIRST_FROST_DAYS} days")),
        )
        .row(PanelRow::new("a good week for sowing roots").status(Status::Success))
        .print()?;
    KeyValue::new()
        .row("made with", format!("paloma {}", env!("CARGO_PKG_VERSION")))
        // The value goes to stdout, so the link is styled for stdout.
        .row(
            "source",
            link("https://github.com/mvilark/paloma")
                .stream(Stream::Stdout)
                .to_string(),
        )
        .print()?;
    Ok(0)
}

pub fn seeds(_: &Args) -> Result<i32> {
    Table::new(["seed", "sow in", "sprouts in"])
        .rows(
            SEEDS
                .iter()
                .map(|s| [s.name.to_string(), s.season.to_string(), s.sprouts_in()]),
        )
        .print()?;
    Ok(0)
}
