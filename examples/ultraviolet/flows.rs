//! One function per command, with canned data.
use std::time::Duration;

use paloma::{
    terminal::Stream, text::link, Confirm, FuzzySelect, Input, KeyValue, Message, MultiSelect,
    Panel, PanelRow, Password, Result, Select, Sort, Status, Table,
};

use crate::{theme, ui, Args};

/// id, place, load in percent, ping in ms
const SERVERS: [(&str, &str, u8, u16); 6] = [
    ("us-nyc-01", "New York, US", 42, 18),
    ("de-fra-02", "Frankfurt, DE", 67, 31),
    ("es-bcn-01", "Barcelona, ES", 23, 12),
    ("jp-tyo-03", "Tokyo, JP", 55, 96),
    ("br-sao-01", "Sao Paulo, BR", 31, 74),
    ("au-syd-02", "Sydney, AU", 48, 121),
];

const PROTOCOLS: [&str; 3] = ["WireGuard", "OpenVPN UDP", "OpenVPN TCP"];
const FEATURES: [&str; 4] = [
    "Kill switch",
    "Split tunnelling",
    "Ad blocking",
    "Auto reconnect",
];

// `validate` needs a function over `&String`, the type `Input` reads.
#[allow(clippy::ptr_arg)]
fn valid_email(s: &String) -> std::result::Result<(), String> {
    match s.split_once('@') {
        Some((user, host)) if !user.is_empty() && host.contains('.') => Ok(()),
        _ => Err("Enter an address like name@example.com".into()),
    }
}

/// Runs a task whose work is faked: the steps print along the way and the whole thing
/// takes about `secs` when there is motion. Returns the time it took.
fn task(running: &str, steps: &[&str], done: &str, secs: f32) -> Result<Duration> {
    let task = paloma::task(running).start()?;
    for step in steps {
        ui::work(secs, steps.len() + 1);
        task.step(*step)?;
    }
    ui::work(secs, steps.len() + 1);
    task.success(done)
}

/// A duration as the panel shows it: `1.4s`.
fn secs(d: Duration) -> String {
    format!("{:.1}s", d.as_secs_f32())
}

pub fn setup(args: &Args) -> Result<i32> {
    const STEPS: usize = 5;
    // The theme draws the `01/05` counter after the label.
    let label = |n: usize, text: &str| {
        theme::set_step(n as u32, STEPS as u32);
        text.to_string()
    };
    let mut step = 0;
    // Earlier answers, kept so going back with Esc shows them again.
    let (mut email, mut protocol, mut features, mut startup) = (String::new(), 0, vec![0, 3], true);

    while step < STEPS {
        let n = step + 1;
        // `None` means the user pressed Esc: go back one prompt. A flag or `--yes` presets
        // the answer, and the prompt only prints the line it would have left.
        let advanced: Option<()> = match step {
            0 => {
                let preset =
                    (args.email.clone()).or_else(|| args.yes.then(|| "ada@example.com".into()));
                email = Input::new()
                    .prompt(label(n, "Account email"))
                    .validate(valid_email)
                    .preset(preset)
                    .interact()?;
                Some(())
            }
            1 => {
                Password::new()
                    .prompt(label(n, "Password"))
                    .preset(args.yes.then(String::new))
                    .interact()?;
                Some(())
            }
            2 => {
                let flag = args
                    .protocol
                    .as_deref()
                    .and_then(|p| PROTOCOLS.iter().position(|x| x.eq_ignore_ascii_case(p)));
                Select::new()
                    .prompt(label(n, "Protocol"))
                    .items(PROTOCOLS)
                    .default(protocol)
                    .preset(flag.or_else(|| args.yes.then_some(protocol)))
                    .interact_opt()?
                    .map(|i| protocol = i)
            }
            3 => {
                let checked: Vec<bool> =
                    (0..FEATURES.len()).map(|i| features.contains(&i)).collect();
                MultiSelect::new()
                    .prompt(label(n, "Features"))
                    .items(FEATURES)
                    .defaults(&checked)
                    .preset(args.yes.then(|| features.clone()))
                    .interact_opt()?
                    .map(|picked| features = picked)
            }
            _ => Confirm::new()
                .prompt(label(n, "Connect on startup?"))
                .default(startup)
                .preset(args.yes.then_some(startup))
                .interact_opt()?
                .map(|v| startup = v),
        };
        match advanced {
            Some(()) => step += 1,
            None => step = step.saturating_sub(1),
        }
    }

    theme::clear_step();
    paloma::section("Setup").print()?;
    let a = task(
        "Creating your profile...",
        &["Generated device key"],
        "Created your profile",
        0.9,
    )?;
    let b = task("Applying settings...", &[], "Applied settings", 0.5)?;
    Panel::new("Setup complete")
        .row(
            PanelRow::new(format!("Signed in as {email}"))
                .status(Status::Success)
                .trailing(secs(a)),
        )
        .row(
            PanelRow::new(format!(
                "{} · {} {}",
                PROTOCOLS[protocol],
                features.len(),
                if features.len() == 1 {
                    "feature"
                } else {
                    "features"
                }
            ))
            .status(Status::Success)
            .trailing(secs(b)),
        )
        .row(
            PanelRow::new(format!(
                "Connect on startup: {}",
                if startup { "On" } else { "Off" }
            ))
            .status(Status::Info),
        )
        .print()?;
    println!("{email}");
    Ok(0)
}

pub fn connect(args: &Args) -> Result<i32> {
    let places: Vec<String> = SERVERS
        .iter()
        .map(|(id, place, ..)| format!("{id} · {place}"))
        .collect();
    let preset = match &args.server {
        Some(id) => match SERVERS.iter().position(|s| s.0 == id) {
            Some(i) => Some(i),
            None => {
                paloma::error(format!("Server {id} not found"))
                    .nest(Message::hint("ultraviolet servers"))
                    .print()?;
                return Ok(1);
            }
        },
        None => args.yes.then_some(0),
    };
    let Some(idx) = FuzzySelect::new()
        .prompt("Server")
        .items(&places)
        .default(0)
        .preset(preset)
        .interact_opt()?
    else {
        return Ok(1);
    };
    let (id, place, load, ping) = SERVERS[idx];

    // The kill switch and disconnect confirms are skipped under `--yes` (no preset: a
    // preset would print an answered line, and the transcripts have none).
    let kill_switch = false;
    if !kill_switch {
        let go = args.yes
            || Confirm::new()
                .prompt("Kill switch is off. Connect anyway?")
                .default(false)
                .interact()?;
        if !go {
            paloma::warning("Not connected").print()?;
            return Ok(0);
        }
    }

    paloma::section("Connect").print()?;
    let t = task(
        &format!("Connecting to {id}..."),
        &["Resolved endpoint", "Opened tunnel", "Negotiated cipher"],
        &format!("Connected to {id}"),
        1.4,
    )?;
    ui::progress("Exchanging keys");
    let mut panel = Panel::new("Connected")
        .row(
            PanelRow::new(format!("{place} · {ping} ms · {load}% load"))
                .status(Status::Success)
                .trailing(secs(t)),
        )
        .row(PanelRow::new("Key exchange complete").status(Status::Success));
    if !kill_switch {
        panel = panel.row(PanelRow::new("Kill switch is off").status(Status::Warning));
    }
    panel.print()?;
    println!("{id}");
    Ok(0)
}

pub fn favorites(args: &Args) -> Result<i32> {
    let items = ["es-bcn-01", "de-fra-02", "us-nyc-01", "jp-tyo-03"];
    // The prompt and the line a preset leaves have always had different labels.
    let label = if args.yes {
        "Favourites"
    } else {
        "Favourite order"
    };
    let Some(order) = Sort::new()
        .prompt(label)
        .items(items)
        .preset(args.yes.then(|| (0..items.len()).collect()))
        .interact_opt()?
    else {
        return Ok(1);
    };
    for i in order {
        println!("{}", items[i]);
    }
    Ok(0)
}

pub fn disconnect(args: &Args) -> Result<i32> {
    let go = args.yes
        || Confirm::new()
            .prompt("Disconnect?")
            .default(true)
            .interact()?;
    if !go {
        return Ok(0);
    }
    task("Disconnecting...", &[], "Disconnected", 0.6)?;
    Ok(0)
}

pub fn status(args: &Args) -> Result<i32> {
    if args.json {
        println!(
            "{{\"state\":\"connected\",\"server\":\"es-bcn-01\",\"protocol\":\"wireguard\",\"ping_ms\":12,\"sent_mb\":4.8,\"received_mb\":31.2}}"
        );
    } else {
        KeyValue::new()
            .row("State", "Connected")
            .row("Server", "es-bcn-01 · Barcelona, ES")
            .row("Protocol", "WireGuard")
            .row("Ping", "12 ms")
            .row("Sent", "4.8 MB")
            .row("Received", "31.2 MB")
            .print()?;
    }
    Ok(0)
}

pub fn servers(_: &Args) -> Result<i32> {
    let rows = SERVERS.iter().map(|(id, place, load, ping)| {
        [
            id.to_string(),
            place.to_string(),
            format!("{load}%"),
            format!("{ping} ms"),
        ]
    });
    Table::new(["Server", "Place", "Load", "Ping"])
        .rows(rows)
        .print()?;
    Ok(0)
}

pub fn about(_: &Args) -> Result<i32> {
    ui::wordmark_with_icon("A fake VPN, themed with Ultraviolet");
    eprintln!();
    KeyValue::new()
        .row("Version", env!("CARGO_PKG_VERSION"))
        .row(
            "Built with",
            format!("paloma {}", env!("CARGO_PKG_VERSION")),
        )
        // The value goes to stdout, so the link is styled for stdout.
        .row(
            "Docs",
            link("https://github.com/mvilark/paloma")
                .stream(Stream::Stdout)
                .to_string(),
        )
        .print()?;
    Ok(0)
}
