//! One function per command. Prompts, flows and messages use content only.
use std::{cell::Cell, error::Error, fs, path::Path};

use chrono::{Duration, Local, NaiveDateTime as DateTime, NaiveTime, Timelike};
use paloma::{
    Confirm, Flow, FuzzySelect, Input, IntoItem, Item, KeyValue, Message, MultiSelect, Select,
    Table,
};
use sunrise::Coordinates;

use crate::{cities::CITIES, schedule::*};

type R = Result<(), Box<dyn Error>>;

const RESTED: [&str; 5] = ["drained", "tired", "okay", "rested", "great"];

/// A `Select` whose highlighted item is `default`.
fn choose(
    prompt: &str,
    items: impl IntoIterator<Item = impl IntoItem>,
    default: usize,
) -> paloma::Result<usize> {
    Select::new()
        .prompt(prompt)
        .items(items)
        .default(default)
        .interact()
}

/// `count` times, half an hour apart.
fn slots(start: NaiveTime, count: i64) -> Vec<NaiveTime> {
    (0..count)
        .map(|i| start + Duration::minutes(30 * i))
        .collect()
}

fn clock(t: impl Timelike) -> String {
    format!("{:02}:{:02}", t.hour(), t.minute())
}

/// Real coordinates from "41.39, 2.17".
fn coordinates(text: &str) -> Option<(f64, f64)> {
    let (lat, lon) = text.split_once(',')?;
    let (lat, lon) = (lat.trim().parse().ok()?, lon.trim().parse().ok()?);
    Coordinates::new(lat, lon).map(|_| (lat, lon))
}

pub fn setup(data: Option<Data>, path: &Path, now: DateTime) -> R {
    let (data, from) = (data.unwrap_or_default(), now.date());
    let sunrise = Cell::new(matches!(data.schedule.wake, Wake::Sunrise(_)));
    let (elsewhere, usual) = (Cell::new(false), Cell::new(None));
    let by_clock = |_: &Data| !sunrise.get();
    let by_sun = |_: &Data| sunrise.get();
    let asked = |_: &Data| elsewhere.get();
    let caffeine = |d: &Data| d.habits.contains(&Habit::Caffeine);
    let find = |list: &[u32], v, other| list.iter().position(|x| *x == v).unwrap_or(other);
    let mut data = Flow::new(data)
        .step("wake", |_| {
            let (set, sun) = (Item::new("at a set time"), Item::new("with the sunrise"));
            let items = [
                set.description("the same time every day, weekends too"),
                sun.description("between 06:00 and 08:00, following the sun where you live"),
            ];
            let now = sunrise.get().into();
            choose("how do you want to wake up?", items, now).map(|i| sunrise.set(i == 1))
        })
        .step_if(by_clock, "time", |d| {
            let times = slots(at(5, 30), 8);
            let now = times.iter().position(|t| Wake::At(*t) == d.schedule.wake);
            let items = times.iter().map(|t| clock(*t));
            choose("wake up at", items, now.unwrap_or(3))
                .map(|i| d.schedule.wake = Wake::At(times[i]))
        })
        .step_if(by_sun, "place", |d| {
            let name = if let Wake::Sunrise(p) = &d.schedule.wake {
                &p.place[..]
            } else {
                ""
            };
            let i = FuzzySelect::new()
                .prompt("where do you live?")
                .placeholder("search cities")
                .items(CITIES.iter().map(|c| format!("{}, {}", c.0, c.1)))
                .item(Item::new("somewhere else").description("enter coordinates"))
                .default(CITIES.iter().position(|c| c.0 == name).unwrap_or(0))
                .interact()?;
            elsewhere.set(i == CITIES.len());
            if let Some(&(place, _, lat, lon)) = CITIES.get(i) {
                let place = place.to_string();
                d.schedule.wake = Wake::Sunrise(Place { place, lat, lon });
            }
            Ok(())
        })
        .step_if(asked, "coordinates", |d| {
            let valid = |t: &String| {
                coordinates(t)
                    .map(drop)
                    .ok_or("two numbers, like 41.39, 2.17")
            };
            let text = Input::<String>::new()
                .prompt("latitude, longitude")
                .description("like 41.39, 2.17")
                .validate(valid)
                .interact()?;
            let (lat, lon) = coordinates(&text).unwrap();
            let place = format!("{lat}, {lon}");
            d.schedule.wake = Wake::Sunrise(Place { place, lat, lon });
            Ok(())
        })
        .step("sleep", |d| {
            let minutes = [420, 450, 480, 510, 540];
            let mut note = "adults need 7 to 9 hours".to_string();
            if sunrise.get() {
                let wake = wake_on(&d.schedule, from.succ_opt().unwrap(), &Local);
                note += &format!(" · tomorrow you wake at {}", clock(wake));
            }
            let i = Select::new()
                .prompt("how much sleep do you need?")
                .description(note)
                .items(minutes.map(hm))
                .default(find(&minutes, d.schedule.sleep_minutes, 2))
                .interact()?;
            d.schedule.sleep_minutes = minutes[i];
            Ok(())
        })
        .step("wind down", |d| {
            let minutes = [30, 45, 60, 90];
            let mut items = minutes.map(|m| Item::new(format!("{m} min")));
            items[2] = items[2].clone().description("good for evening hyperfocus");
            let i = Select::new()
                .prompt("wind down for")
                .description("a hard stop for screens and work")
                .items(items)
                .default(find(&minutes, d.schedule.wind_down_minutes, 2))
                .interact()?;
            d.schedule.wind_down_minutes = minutes[i];
            Ok(())
        })
        .step("usual", |_| {
            let times = slots(at(22, 30), 11);
            let mut items: Vec<_> = times.iter().map(|t| clock(*t)).collect();
            (items[0], items[10]) = ("around the time above".into(), "later than 03:00".into());
            choose("when do you fall asleep these days?", items, 0)
                .map(|i| usual.set((i > 0).then(|| times[i])))
        })
        .step("habits", |d| {
            let items = Habit::ALL.map(|h| {
                (
                    Item::new(h.text()).description(h.why()),
                    d.habits.contains(&h),
                )
            });
            let none = |v: &[usize]| {
                (!v.is_empty())
                    .then_some(())
                    .ok_or("pick at least one habit")
            };
            let picked = MultiSelect::new()
                .prompt("which habits will you keep?")
                .items_checked(items)
                .validate(none)
                .interact()?;
            d.habits = picked.into_iter().map(|i| Habit::ALL[i]).collect();
            Ok(())
        })
        .step_if(caffeine, "caffeine", |d| {
            let hours = [6, 8, 10];
            let now = find(&hours, d.schedule.caffeine_hours.unwrap_or(8), 1);
            let items = hours.map(|h| format!("{h} hours before bed"));
            choose("last caffeine", items, now).map(|i| d.schedule.caffeine_hours = Some(hours[i]))
        })
        .run()?;

    if !data.habits.contains(&Habit::Caffeine) {
        data.schedule.caffeine_hours = None;
    }
    data.schedule.ease = None;
    let target = plan(&data.schedule, from, &Local).bed.time();
    let ease = usual.get().and_then(|u| ease_for(target, u));
    data.schedule.ease = ease.map(|minutes| Ease { minutes, from });
    save(path, &data)?;

    let (s, plan) = (&data.schedule, plan(&data.schedule, from, &Local));
    let wake = match &s.wake {
        Wake::At(t) => clock(*t),
        Wake::Sunrise(p) => format!("with the sunrise in {}, 06:00-08:00", p.place),
    };
    let habits: Vec<_> = data.habits.iter().map(|h| h.text()).collect();
    let caffeine = plan
        .caffeine
        .map(|c| ("caffeine", format!("until {}", clock(c))));
    paloma::section("your schedule").print()?;
    KeyValue::new()
        .rows([
            ("wake", wake),
            ("sleep", hm(s.sleep_minutes)),
            ("wind down", format!("{} min", s.wind_down_minutes)),
        ])
        .rows(caffeine)
        .row("habits", habits.join(", "))
        .print()?;
    paloma::success("saved")
        .nest(Message::hint(path.display().to_string()))
        .print()?;
    if s.ease.is_some() {
        let bed = clock(plan.bed);
        paloma::info(format!(
            "starting at {bed}, the whole night moves 15 min earlier every 3 days"
        ))
        .print()?;
    }
    Ok(())
}

pub fn today(data: &Data, now: DateTime) -> R {
    let s = &data.schedule;
    let (which, tonight) = phase(s, now, &Local);
    let span = |from: DateTime, to: DateTime| hm((to - from).num_minutes());
    let sunrise = ["", " (sunrise)"][matches!(s.wake, Wake::Sunrise(_)) as usize];
    paloma::section("tonight").print()?;
    KeyValue::new()
        .rows(tonight.caffeine.map(|c| ("caffeine until", clock(c))))
        .rows([
            ("wind down", clock(tonight.wind_down)),
            ("bed", clock(tonight.bed)),
            ("wake", clock(tonight.wake) + sunrise),
        ])
        .print()?;
    match which {
        Phase::Day => paloma::info(format!("wind-down in {}", span(now, tonight.wind_down))),
        Phase::WindDown => {
            let hint = ["screens off", "park your work: lull park"]
                [data.habits.contains(&Habit::Park) as usize];
            paloma::warning(format!(
                "wind-down started {} ago",
                span(tonight.wind_down, now)
            ))
            .nest(Message::hint(hint))
        }
        Phase::Night if now < tonight.bed + Duration::minutes(10) => paloma::info("time to sleep"),
        Phase::Night => paloma::warning(format!("{} past bedtime", span(tonight.bed, now)))
            .nest(Message::hint("it will all still be there tomorrow")),
    }
    .print()?;
    if tonight.ease > 0 {
        let night = tonight.wake.date().pred_opt().unwrap();
        let next = (1..)
            .map(|i| night + Duration::days(i))
            .find(|d| ease_minutes(s, *d) < tonight.ease);
        let note = format!(
            "easing in: {} later than your goal, 15 min earlier on {}",
            hm(tonight.ease),
            next.unwrap().format("%b %-d")
        );
        Message::hint(note).print()?;
    }
    let last = night_date(now);
    let woke = now >= plan(s, last, &Local).wake && now.time() < at(12, 0);
    if woke && !data.nights.iter().any(|n| n.date == last) {
        Message::hint("good morning: lull log").print()?;
    }
    Ok(())
}

pub fn park(mut data: Data, path: &Path, now: DateTime) -> R {
    let blank = |t: &String| {
        (!t.trim().is_empty())
            .then_some(())
            .ok_or("a few words is enough")
    };
    let note = Input::<String>::new()
        .prompt("where did you stop?")
        .placeholder("the next thing to do, in a few words")
        .validate(blank)
        .interact()?;
    data.parked = Some(Parked {
        at: now,
        note: note.trim().into(),
    });
    save(path, &data)?;
    paloma::success("parked")
        .nest(Message::hint("it will be here in the morning"))
        .print()?;
    Ok(())
}

pub fn log(mut data: Data, path: &Path, now: DateTime) -> R {
    let date = night_date(now);
    let planned = plan(&data.schedule, date, &Local);
    let parse = |t: &str| NaiveTime::parse_from_str(t, "%H:%M");
    let valid = |t: &String| parse(t).map(drop).map_err(|_| "a time like 23:30");
    let ask = |prompt: &str, default: NaiveTime| -> paloma::Result<NaiveTime> {
        let text = Input::<String>::new()
            .prompt(prompt)
            .default(clock(default))
            .validate(valid)
            .interact()?;
        Ok(parse(&text).unwrap())
    };
    let chosen = data.habits.clone();
    let start = Night {
        date,
        bed: planned.bed.time(),
        up: planned.wake.time(),
        rested: 3,
        habits: vec![],
    };
    let night = Flow::new(start)
        .step("bed", |n| ask("went to bed at", n.bed).map(|t| n.bed = t))
        .step("up", |n| ask("got up at", n.up).map(|t| n.up = t))
        .step("rested", |n| {
            choose("how rested do you feel?", RESTED, 2).map(|i| n.rested = i as u8 + 1)
        })
        .step_if(
            |_| !chosen.is_empty(),
            "habits",
            |n| {
                let kept = MultiSelect::new()
                    .prompt("which habits did you keep?")
                    .items(chosen.iter().map(|h| h.text()));
                kept.interact()
                    .map(|v| n.habits = v.into_iter().map(|i| chosen[i]).collect())
            },
        )
        .run()?;
    let slept = (night.up - night.bed).num_minutes().rem_euclid(1440);
    let late = minutes_between(planned.bed.time(), night.bed);
    data.nights.retain(|n| n.date != date);
    data.nights.push(night);
    data.nights.sort_by_key(|n| n.date);
    let parked = data.parked.take();
    save(path, &data)?;
    if let Some(p) = parked {
        paloma::info("you parked:")
            .nest(Message::info(p.note))
            .print()?;
    }
    paloma::success(format!("logged {}", hm(slept))).print()?;
    if late > 30 {
        let hint = Message::hint("start the wind-down on time tonight");
        paloma::warning(format!("{} later than planned", hm(late)))
            .nest(hint)
            .print()?;
    }
    Ok(())
}

pub fn week(data: &Data) -> R {
    let last = &data.nights[data.nights.len().saturating_sub(7)..];
    if last.is_empty() {
        paloma::info("no nights logged yet")
            .nest(Message::hint("lull log, in the morning"))
            .print()?;
        return Ok(());
    }
    let slept = |n: &Night| (n.up - n.bed).num_minutes().rem_euclid(1440);
    let rows = last.iter().map(|n| {
        let habits = format!("{}/{}", n.habits.len(), data.habits.len());
        [
            n.date.format("%a %-d").to_string(),
            clock(n.bed),
            clock(n.up),
            hm(slept(n)),
            RESTED[n.rested as usize - 1].into(),
            habits,
        ]
    });
    Table::new(["night", "bed", "up", "slept", "rested", "habits"])
        .rows(rows)
        .print()?;
    let spread = spread(&last.iter().map(|n| n.bed).collect::<Vec<_>>());
    let near = |n: &&Night| {
        minutes_between(plan(&data.schedule, n.date, &Local).bed.time(), n.bed).abs() <= 30
    };
    let kept = last.iter().map(|n| n.habits.len()).sum::<usize>() as f64;
    KeyValue::new()
        .row(
            "average sleep",
            hm(last.iter().map(slept).sum::<i64>() / last.len() as i64),
        )
        .row("bedtime spread", hm(spread))
        .row(
            "on time",
            format!("{} of {}", last.iter().filter(near).count(), last.len()),
        )
        .row(
            "habits kept",
            format!(
                "{}%",
                (100.0 * kept / (last.len() * data.habits.len()) as f64).round()
            ),
        )
        .print()?;
    if spread <= 60 {
        paloma::success("a steady week").print()?;
    } else {
        let hint = Message::hint("the wind-down is the anchor: lull today");
        paloma::warning(format!("your bedtime moved by {} this week", hm(spread)))
            .nest(hint)
            .print()?;
    }
    Ok(())
}

pub fn forget(path: &Path) -> R {
    if Confirm::new()
        .prompt(format!("delete {}?", path.display()))
        .default(false)
        .interact()?
    {
        fs::remove_file(path)?;
        paloma::success("forgotten").print()?;
    } else {
        paloma::info("kept").print()?;
    }
    Ok(())
}
