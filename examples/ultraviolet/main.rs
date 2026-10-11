//! A fake VPN CLI themed with the Ultraviolet design system (docs/design/ultraviolet.md).
//!
//! Run with: `cargo run --example ultraviolet --all-features -- <command>`
//! Commands: setup, connect, favorites, disconnect, status, servers, about.
mod flows;
mod theme;
mod ui;

use std::env;

use console::Term;
use theme::{Config, Depth, Glyphs, Palette, UltravioletTheme};

#[derive(Default)]
pub struct Args {
    pub command: Option<String>,
    pub theme: Option<String>,
    pub ascii: bool,
    pub yes: bool,
    pub json: bool,
    pub email: Option<String>,
    pub protocol: Option<String>,
    pub server: Option<String>,
}

fn parse_args() -> Args {
    let mut args = Args::default();
    let mut it = env::args().skip(1);
    while let Some(arg) = it.next() {
        let (key, inline) = match arg.split_once('=') {
            Some((k, v)) => (k.to_string(), Some(v.to_string())),
            None => (arg.clone(), None),
        };
        let mut value = || inline.clone().or_else(|| it.next());
        match key.as_str() {
            "--theme" => args.theme = value(),
            "--ascii" => args.ascii = true,
            "--yes" | "-y" => args.yes = true,
            "--json" => args.json = true,
            "--email" => args.email = value(),
            "--protocol" => args.protocol = value(),
            "--server" => args.server = value(),
            _ if !key.starts_with('-') && args.command.is_none() => args.command = Some(key),
            _ => {}
        }
    }
    args
}

/// Flag, then `ULTRAVIOLET_THEME`, then `COLORFGBG`, otherwise dark.
/// (The guide's OSC 11 background query is not done here; see the PAL-002 gap list.)
fn is_dark(args: &Args) -> bool {
    let named = args
        .theme
        .clone()
        .or_else(|| env::var("ULTRAVIOLET_THEME").ok());
    match named.as_deref() {
        Some("light") => return false,
        Some("dark") => return true,
        _ => {}
    }
    match env::var("COLORFGBG")
        .ok()
        .and_then(|v| v.rsplit(';').next().and_then(|bg| bg.parse::<u8>().ok()))
    {
        Some(bg) => !matches!(bg, 7 | 15),
        None => true,
    }
}

fn depth() -> Depth {
    if matches!(env::var("COLORTERM").as_deref(), Ok("truecolor" | "24bit")) {
        Depth::True
    } else if env::var("TERM").is_ok_and(|t| t.contains("256color")) {
        Depth::C256
    } else {
        Depth::C16
    }
}

fn main() {
    let mut args = parse_args();
    let tty = Term::stderr().is_term();
    let ascii = args.ascii || env::var("TERM").is_ok_and(|t| t == "linux");
    let motion =
        tty && env::var_os("CI").is_none() && env::var("ULTRAVIOLET_MOTION").as_deref() != Ok("0");
    // Prompts need a terminal, so unattended runs accept every default.
    args.yes |= !tty;

    let config = Config {
        pal: Palette::new(is_dark(&args), depth()),
        g: Glyphs::new(ascii),
        motion,
    };
    theme::init(config.clone());
    paloma::set_theme(UltravioletTheme::new(config));
    // How prompts work, as opposed to how they look (the theme).
    paloma::set_options(
        paloma::Options::new()
            .confirm(|c| c.choices_as_buttons(true).on_shortcut(false))
            .password(|p| p.mask(true).result_len(8)),
    );

    // Ctrl+C while a spinner runs (no prompt is reading keys then).
    let _ = ctrlc::set_handler(|| {
        ui::cancelled();
        std::process::exit(130);
    });

    let command = match args.command.as_deref() {
        Some(c) => c.to_string(),
        None => {
            ui::wordmark_with_icon("a fake vpn, themed with Ultraviolet");
            ui::blank();
            "setup".into()
        }
    };
    let result = match command.as_str() {
        "setup" => flows::setup(&args),
        "connect" => flows::connect(&args),
        "favorites" => flows::favorites(&args),
        "disconnect" => flows::disconnect(&args),
        "status" => flows::status(&args),
        "servers" => flows::servers(&args),
        "about" => flows::about(&args),
        other => {
            ui::log(ui::Kind::Err, &format!("Unknown command {other}"));
            ui::hint("ultraviolet about");
            std::process::exit(2);
        }
    };
    let code = match result {
        Ok(code) => code,
        Err(err) if flows::is_cancel(&err) => {
            ui::cancelled();
            130
        }
        Err(err) => {
            ui::log(ui::Kind::Err, &err.to_string());
            1
        }
    };
    std::process::exit(code);
}
