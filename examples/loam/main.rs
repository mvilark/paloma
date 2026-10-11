//! Loam, a cozy garden journal: a second full example app beside Ultraviolet, built only
//! from what paloma offers. It has no UI code of its own (no escape codes, no styled
//! text, no layout): `theme.rs` builds the design as tokens, everything shown is a
//! paloma prompt, flow, message, block or task. All data is canned and lives in memory.
//!
//! Run with: `cargo run --example loam --all-features -- <command>`
//! Commands: init, plant, water, harvest, journal, status, almanac, seeds.
//! Flags: --yes, --ascii, --day, --night, --plain, --name, --zone, --json, --long.
mod commands;
mod data;
mod theme;

use std::env;

use paloma::{
    terminal::{self, Stream},
    theme::SimpleTheme,
    Message, Options,
};
use theme::LoamTheme;

/// What the command line asked for, and what the terminal allows.
#[derive(Default)]
pub struct Args {
    pub command: Option<String>,
    /// Answer every prompt with its default or the flags.
    pub yes: bool,
    pub ascii: bool,
    /// The night palette (the default) or the day one.
    pub night: bool,
    /// Draw with paloma's `SimpleTheme`: Loam has no look of its own to fall back on.
    pub plain: bool,
    pub name: Option<String>,
    pub zone: Option<String>,
    pub json: bool,
    /// `journal` opens the editor.
    pub long: bool,
    /// Whether the faked work really waits and spinners turn.
    pub motion: bool,
    unknown: Vec<String>,
}

fn parse_args() -> Args {
    let mut args = Args {
        night: env::var("LOAM_THEME").as_deref() != Ok("day"),
        ..Args::default()
    };
    let mut it = env::args().skip(1);
    while let Some(arg) = it.next() {
        let (key, inline) = match arg.split_once('=') {
            Some((k, v)) => (k.to_string(), Some(v.to_string())),
            None => (arg.clone(), None),
        };
        let mut value = || inline.clone().or_else(|| it.next());
        match key.as_str() {
            "--yes" | "-y" => args.yes = true,
            "--ascii" => args.ascii = true,
            "--day" => args.night = false,
            "--night" => args.night = true,
            "--plain" => args.plain = true,
            "--json" => args.json = true,
            "--long" => args.long = true,
            "--name" => args.name = value(),
            "--zone" => args.zone = value(),
            _ if key.starts_with('-') => args.unknown.push(key),
            _ if args.command.is_none() => args.command = Some(key),
            _ => args.unknown.push(key),
        }
    }
    args
}

/// What `Ctrl+C` prints, on its own line, before the app exits with code 130.
fn cancel_message() -> Message<'static> {
    Message::hint("put your trowel down - nothing was changed")
}

fn main() {
    let mut args = parse_args();
    let caps = terminal::capabilities(Stream::Stderr);
    // Prompts need a terminal, so unattended runs accept every default.
    args.yes |= !caps.is_tty();
    args.ascii |= !caps.unicode();
    args.motion = caps.is_tty() && env::var_os("CI").is_none();

    if args.plain {
        paloma::set_theme(SimpleTheme);
    } else {
        paloma::set_theme(LoamTheme::new(args.night, caps.depth(), args.ascii));
    }
    // How things work, as opposed to how they look (the theme).
    paloma::set_options(
        Options::new()
            .general(|g| g.unattended(args.yes).cancel_message(cancel_message()))
            .confirm(|c| c.choices_as_buttons(true))
            .password(|p| p.mask(true))
            .live(|l| l.motion(args.motion)),
    );

    // `Ctrl+C` while a spinner runs: no prompt is reading keys then, so paloma cannot
    // see it. A prompt reads `^C` as a key and exits by itself.
    let _ = ctrlc::set_handler(|| {
        paloma::terminal::restore();
        let _ = cancel_message().print();
        std::process::exit(130);
    });

    if let Some(flag) = args.unknown.first() {
        let _ = paloma::error(format!("i do not know {flag}"))
            .nest(Message::hint("loam almanac"))
            .print();
        std::process::exit(2);
    }
    let command = match args.command.as_deref() {
        Some(c) => c.to_string(),
        None => {
            // The first run: the wordmark, then a new garden.
            if let Err(err) = commands::wordmark(&args) {
                let _ = paloma::error(err.to_string()).print();
            }
            "init".into()
        }
    };
    let result = match command.as_str() {
        "init" => commands::init(&args),
        "plant" => commands::plant(&args),
        "water" => commands::water(&args),
        "harvest" => commands::harvest(&args),
        "journal" => commands::journal(&args),
        "status" => commands::status(&args),
        "almanac" => commands::almanac(&args),
        "seeds" => commands::seeds(&args),
        other => {
            let _ = paloma::error(format!("i do not know {other}"))
                .nest(Message::hint("loam almanac"))
                .print();
            std::process::exit(2);
        }
    };
    let code = match result {
        Ok(code) => code,
        Err(err) => {
            let _ = paloma::error(err.to_string()).print();
            1
        }
    };
    std::process::exit(code);
}
