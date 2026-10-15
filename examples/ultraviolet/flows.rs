//! One function per command, with canned data, and the fake work that stands in for the
//! real thing. Everything shown is a paloma component.
use std::{thread, time::Duration};

use paloma::{
    terminal::Stream, text::link, Confirm, Flow, FuzzySelect, Input, KeyValue, Message,
    MultiSelect, Panel, PanelRow, Password, Result, Select, Sort, Status, Table,
};

use crate::{theme, Args};

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

/// Fake work: waits `ms` when there is motion. Without motion nothing waits, so every
/// duration reads `0.0s`.
fn pause(ms: u64) {
    if theme::cfg().motion {
        thread::sleep(Duration::from_millis(ms));
    }
}

/// Fake work: waits `secs` in total, split into `parts` pauses, when there is motion.
fn work(secs: f32, parts: usize) {
    pause((secs * 1000.0) as u64 / parts.max(1) as u64);
}

/// Runs a task whose work is faked: the steps print along the way and the whole thing
/// takes about `secs` when there is motion. Returns the time it took.
fn task(running: &str, steps: &[&str], done: &str, secs: f32) -> Result<Duration> {
    let task = paloma::task(running).start()?;
    for step in steps {
        work(secs, steps.len() + 1);
        task.step(*step)?;
    }
    work(secs, steps.len() + 1);
    task.success(done)
}

/// A duration as the panel shows it: `1.4s`.
fn secs(d: Duration) -> String {
    format!("{:.1}s", d.as_secs_f32())
}

/// What the setup flow asks for. Earlier answers stay here when the user goes back
/// with Esc, so each step shows them again.
struct Setup {
    email: String,
    protocol: usize,
    features: Vec<usize>,
    startup: bool,
}

pub fn setup(args: &Args) -> Result<i32> {
    // The flow numbers the steps (the theme draws `01/05` after the label), goes back
    // a step on Esc and cancels on Esc at the first one. A flag or `--yes` presets an
    // answer, and the prompt only prints the line it would have left.
    let Setup {
        email,
        protocol,
        features,
        startup,
    } = Flow::new(Setup {
        email: String::new(),
        protocol: 0,
        features: vec![0, 3],
        startup: true,
    })
    .step("email", |s| {
        let preset = args
            .email
            .clone()
            .or_else(|| args.yes.then(|| "ada@example.com".into()));
        s.email = Input::new()
            .prompt("Account email")
            .prefill(s.email.clone())
            .validate(valid_email)
            .preset(preset)
            .interact()?;
        Ok(())
    })
    .step("password", |_| {
        Password::new()
            .prompt("Password")
            .preset(args.yes.then(String::new))
            .interact()?;
        Ok(())
    })
    .step("protocol", |s| {
        let flag = args
            .protocol
            .as_deref()
            .and_then(|p| PROTOCOLS.iter().position(|x| x.eq_ignore_ascii_case(p)));
        s.protocol = Select::new()
            .prompt("Protocol")
            .items(PROTOCOLS)
            .default(s.protocol)
            .preset(flag.or_else(|| args.yes.then_some(s.protocol)))
            .interact()?;
        Ok(())
    })
    .step("features", |s| {
        let checked: Vec<bool> = (0..FEATURES.len())
            .map(|i| s.features.contains(&i))
            .collect();
        s.features = MultiSelect::new()
            .prompt("Features")
            .items(FEATURES)
            .defaults(&checked)
            .preset(args.yes.then(|| s.features.clone()))
            .interact()?;
        Ok(())
    })
    .step("startup", |s| {
        s.startup = Confirm::new()
            .prompt("Connect on startup?")
            .default(s.startup)
            .preset(args.yes.then_some(s.startup))
            .interact()?;
        Ok(())
    })
    .run()?;

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

/// What the connect flow asks for.
struct Connect {
    server: usize,
    go: bool,
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
    // A flow too: Esc on the kill-switch question goes back to the server list, with
    // the server picked before still highlighted. Under `--yes` the question is
    // skipped (a preset would print an answered line, and the transcripts have none).
    let kill_switch = false;
    let Connect { server, go } = Flow::new(Connect {
        server: 0,
        go: true,
    })
    .step("server", |s| {
        s.server = FuzzySelect::new()
            .prompt("Server")
            .items(&places)
            .default(s.server)
            .preset(preset)
            .interact()?;
        Ok(())
    })
    .step_if(
        |_| !kill_switch && !args.yes,
        "kill switch",
        |s| {
            s.go = Confirm::new()
                .prompt("Kill switch is off. Connect anyway?")
                .default(false)
                .interact()?;
            Ok(())
        },
    )
    .run()?;
    if !go {
        paloma::warning("Not connected").print()?;
        return Ok(0);
    }
    let (id, place, load, ping) = SERVERS[server];

    paloma::section("Connect").print()?;
    let t = task(
        &format!("Connecting to {id}..."),
        &["Resolved endpoint", "Opened tunnel", "Negotiated cipher"],
        &format!("Connected to {id}"),
        1.4,
    )?;
    // 32 cells, each a little slower than the one before (eased: fast at first, slower
    // toward the end); the bar eases between them and stays on screen when done.
    let bar = paloma::progress("Exchanging keys", 32).start()?;
    for cell in 0..32 {
        pause(45 + cell * 40 / 32);
        bar.inc(1)?;
    }
    bar.keep()?;
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
    theme::wordmark_with_icon("A fake VPN, themed with Ultraviolet");
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

// ---------------------------------------------------------------------------------
// Help: the one place that lists the commands, options and environment variables.
// Keep it in sync with `parse_args` and `main` in `main.rs`.

const USAGE: [(&str, &str); 2] = [
    ("ultraviolet <command> [options]", "Run a command"),
    ("ultraviolet", "Same as setup, after the wordmark"),
];

const HELP_COMMANDS: [(&str, &str); 8] = [
    ("setup", "Sign in and choose your protocol and features"),
    ("connect", "Pick a server and connect to it"),
    ("favorites", "Put your favourite servers in order"),
    ("disconnect", "Close the connection"),
    ("status", "Show the state of the connection"),
    ("servers", "List the servers with their load and ping"),
    ("about", "Show the version and the docs link"),
    ("help", "Show this help"),
];

const HELP_OPTIONS: [(&str, &str); 8] = [
    ("--yes, -y", "Accept every default and ask nothing"),
    ("--email <email>", "Account email for setup"),
    (
        "--protocol <name>",
        "Protocol for setup: WireGuard, OpenVPN UDP or OpenVPN TCP",
    ),
    ("--server <id>", "Server for connect, such as es-bcn-01"),
    ("--json", "Print status as JSON"),
    (
        "--theme <dark|light>",
        "Colours for a dark or a light terminal",
    ),
    ("--ascii", "Draw with ASCII characters only"),
    ("--help, -h", "Show this help"),
];

const HELP_ENVIRONMENT: [(&str, &str); 6] = [
    (
        "ULTRAVIOLET_THEME",
        "dark or light, when --theme is not given",
    ),
    ("ULTRAVIOLET_MOTION", "0 keeps spinners and bars still"),
    ("COLORFGBG", "Tells a light terminal from a dark one"),
    ("COLORTERM", "truecolor or 24bit draws in 24-bit colour"),
    (
        "TERM",
        "256color draws in 256 colours; linux draws in ASCII",
    ),
    ("CI", "When set, spinners and bars stay still"),
];

pub fn help(_: &Args) -> Result<i32> {
    let groups: [(&str, &[(&str, &str)]); 4] = [
        ("Usage", &USAGE),
        ("Commands", &HELP_COMMANDS),
        ("Options", &HELP_OPTIONS),
        ("Environment", &HELP_ENVIRONMENT),
    ];
    for (title, rows) in groups {
        paloma::section(title).print()?;
        KeyValue::new().rows(rows.iter().copied()).print()?;
    }
    Ok(0)
}
