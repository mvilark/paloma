//! Lull, a sleep schedule tracker for people whose bedtime drifts because they hyperfocus
//! in the evening. It saves to one JSON file, shows paloma's defaults everywhere, and
//! writes content and two themes: the night one from the wind-down until the wake time.
//!
//! Run with: `cargo run --example lull --features fuzzy-select -- <command>`
//! Commands: setup, today, park, log, week, forget. Environment: LULL_FILE, LULL_NOW.
mod cities;
mod commands;
mod schedule;
mod theme;

use std::{env, error::Error, process::exit};

use chrono::{Local, NaiveDateTime};
use paloma::{terminal, theme::SimpleTheme, Message};
use schedule::{Data, Phase};

const NOW: &str = "%Y-%m-%dT%H:%M";

/// Prints `message` and exits with `code`.
fn fail(message: Message, code: i32) -> ! {
    let _ = message.print();
    exit(code)
}

fn main() {
    if let Err(err) = run() {
        fail(paloma::error(err.to_string()), 1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let now = env::var("LULL_NOW").unwrap_or_else(|_| Local::now().format(NOW).to_string());
    let now = NaiveDateTime::parse_from_str(&now, NOW).map_err(|e| format!("LULL_NOW: {e}"))?;
    let path = schedule::path();
    let data =
        schedule::load(&path).map_err(|e| format!("{} could not be read: {e}", path.display()))?;

    // Without a schedule yet, the default one decides the theme.
    let caps = terminal::capabilities(terminal::Stream::Stderr);
    let schedule = data
        .as_ref()
        .map_or_else(|| Data::default().schedule, |d| d.schedule.clone());
    if !caps.unicode() {
        paloma::set_theme(SimpleTheme);
    } else if schedule::phase(&schedule, now, &Local).0 == Phase::Day {
        paloma::set_theme(theme::day(caps.depth()));
    } else {
        paloma::set_theme(theme::night(caps.depth()));
    }

    let default = if data.is_none() && caps.is_tty() {
        "setup"
    } else {
        "today"
    };
    match (env::args().nth(1).as_deref().unwrap_or(default), data) {
        ("setup", data) => commands::setup(data, &path, now),
        ("today" | "park" | "log" | "week" | "forget", None) => fail(
            paloma::error("no sleep schedule yet").nest(Message::hint("run: lull setup")),
            1,
        ),
        ("today", Some(data)) => commands::today(&data, now),
        ("park", Some(data)) => commands::park(data, &path, now),
        ("log", Some(data)) => commands::log(data, &path, now),
        ("week", Some(data)) => commands::week(&data),
        ("forget", Some(_)) => commands::forget(&path),
        (other, _) => {
            let hint = Message::hint("lull setup, today, park, log, week or forget");
            fail(
                paloma::error(format!("i do not know {other}")).nest(hint),
                2,
            )
        }
    }
}
