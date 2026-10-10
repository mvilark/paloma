//! One function per command, with canned data.
use paloma::{Confirm, Error, FuzzySelect, Input, MultiSelect, Password, Result, Select, Sort};

use crate::{
    theme,
    ui::{self, Kind},
    Args,
};

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

// `validate_with` needs a function over `&String`, the type `Input` reads.
#[allow(clippy::ptr_arg)]
fn valid_email(s: &String) -> std::result::Result<(), String> {
    match s.split_once('@') {
        Some((user, host)) if !user.is_empty() && host.contains('.') => Ok(()),
        _ => Err("Enter an address like name@example.com".into()),
    }
}

pub fn setup(args: &Args) -> Result<i32> {
    const STEPS: usize = 5;
    // The theme draws the `01/05` counter after the label.
    let label = |n: usize, text: &str| {
        theme::set_step(n as u32, STEPS as u32);
        text.to_string()
    };
    let mut step = 0;
    let (mut email, mut protocol, mut features, mut startup) = (String::new(), 0, Vec::new(), true);

    while step < STEPS {
        let n = step + 1;
        // `None` means the user pressed Esc: go back one prompt.
        let advanced: Option<()> = match step {
            0 => {
                let name = label(n, "Account email");
                if let Some(preset) = &args.email {
                    email = preset.clone();
                    ui::answered(&name, &email);
                } else if args.yes {
                    email = "ada@example.com".into();
                    ui::answered(&name, &email);
                } else {
                    email = Input::new()
                        .with_prompt(name)
                        .validate_with(valid_email)
                        .interact_text()?;
                }
                Some(())
            }
            1 => {
                let name = label(n, "Password");
                if args.yes {
                    ui::answered(&name, &theme::cfg().g.dot.repeat(8));
                } else {
                    Password::new().with_prompt(name).interact()?;
                }
                Some(())
            }
            2 => {
                let name = label(n, "Protocol");
                if let Some(preset) = args
                    .protocol
                    .as_deref()
                    .and_then(|p| PROTOCOLS.iter().position(|x| x.eq_ignore_ascii_case(p)))
                {
                    protocol = preset;
                    ui::answered(&name, PROTOCOLS[protocol]);
                    Some(())
                } else if args.yes {
                    ui::answered(&name, PROTOCOLS[0]);
                    Some(())
                } else {
                    Select::new()
                        .with_prompt(name)
                        .items(PROTOCOLS)
                        .default(protocol)
                        .interact_opt()?
                        .map(|i| protocol = i)
                }
            }
            3 => {
                let name = label(n, "Features");
                if args.yes {
                    features = vec![0, 3];
                    ui::answered(&name, "Kill switch, Auto reconnect");
                    Some(())
                } else {
                    let defaults = [true, false, false, true];
                    MultiSelect::new()
                        .with_prompt(name)
                        .items(FEATURES)
                        .defaults(&defaults)
                        .interact_opt()?
                        .map(|picked| features = picked)
                }
            }
            _ => {
                let name = label(n, "Connect on startup?");
                if args.yes {
                    ui::answered(&name, "Yes");
                    Some(())
                } else {
                    Confirm::new()
                        .with_prompt(name)
                        .default(true)
                        .interact_opt()?
                        .map(|v| startup = v)
                }
            }
        };
        match advanced {
            Some(()) => step += 1,
            None => step = step.saturating_sub(1),
        }
    }

    theme::clear_step();
    ui::blank();
    ui::section("Setup");
    let a = ui::task(
        "Creating your profile...",
        &["Generated device key"],
        "Created your profile",
        Kind::Ok,
        0.9,
    );
    let b = ui::task(
        "Applying settings...",
        &[],
        "Applied settings",
        Kind::Ok,
        0.5,
    );
    ui::blank();
    ui::outro(
        "Setup complete",
        &[
            (Kind::Ok, format!("Signed in as {email}"), Some(a)),
            (
                Kind::Ok,
                format!("{} · {} features", PROTOCOLS[protocol], features.len()),
                Some(b),
            ),
            (
                Kind::Info,
                format!("Connect on startup: {}", if startup { "On" } else { "Off" }),
                None,
            ),
        ],
    );
    println!("{email}");
    Ok(0)
}

pub fn connect(args: &Args) -> Result<i32> {
    let places: Vec<String> = SERVERS
        .iter()
        .map(|(id, place, ..)| format!("{id} · {place}"))
        .collect();
    let idx = match &args.server {
        Some(id) => match SERVERS.iter().position(|s| s.0 == id) {
            Some(i) => {
                ui::answered("Server", &places[i]);
                i
            }
            None => {
                ui::log(Kind::Err, &format!("Server {id} not found"));
                ui::hint("ultraviolet servers");
                return Ok(1);
            }
        },
        None if args.yes => {
            ui::answered("Server", &places[0]);
            0
        }
        None => match FuzzySelect::new()
            .with_prompt("Server")
            .items(&places)
            .default(0)
            .interact_opt()?
        {
            Some(i) => i,
            None => return Ok(1),
        },
    };
    let (id, place, load, ping) = SERVERS[idx];

    let kill_switch = false;
    if !kill_switch {
        let go = args.yes
            || Confirm::new()
                .with_prompt("Kill switch is off. Connect anyway?")
                .default(false)
                .interact()?;
        if !go {
            ui::log(Kind::Warn, "Not connected");
            return Ok(0);
        }
    }

    ui::blank();
    ui::section("Connect");
    let t = ui::task(
        &format!("Connecting to {id}..."),
        &["Resolved endpoint", "Opened tunnel", "Negotiated cipher"],
        &format!("Connected to {id}"),
        Kind::Ok,
        1.4,
    );
    ui::progress("Exchanging keys");
    ui::blank();
    let mut rows = vec![
        (
            Kind::Ok,
            format!("{place} · {ping} ms · {load}% load"),
            Some(t),
        ),
        (Kind::Ok, "Key exchange complete".to_string(), None),
    ];
    if !kill_switch {
        rows.push((Kind::Warn, "Kill switch is off".to_string(), None));
    }
    ui::outro("Connected", &rows);
    println!("{id}");
    Ok(0)
}

pub fn favorites(args: &Args) -> Result<i32> {
    let items = ["es-bcn-01", "de-fra-02", "us-nyc-01", "jp-tyo-03"];
    let order: Vec<usize> = if args.yes {
        ui::answered("Favourites", &items.join(", "));
        (0..items.len()).collect()
    } else {
        match Sort::new()
            .with_prompt("Favourite order")
            .items(items)
            .interact_opt()?
        {
            Some(order) => order,
            None => return Ok(1),
        }
    };
    for i in order {
        println!("{}", items[i]);
    }
    Ok(0)
}

pub fn disconnect(args: &Args) -> Result<i32> {
    let go = args.yes
        || Confirm::new()
            .with_prompt("Disconnect?")
            .default(true)
            .interact()?;
    if !go {
        return Ok(0);
    }
    ui::task("Disconnecting...", &[], "Disconnected", Kind::Ok, 0.6);
    Ok(0)
}

pub fn status(args: &Args) -> Result<i32> {
    if args.json {
        println!(
            "{{\"state\":\"connected\",\"server\":\"es-bcn-01\",\"protocol\":\"wireguard\",\"ping_ms\":12,\"sent_mb\":4.8,\"received_mb\":31.2}}"
        );
    } else {
        ui::kv(&[
            ("State", "Connected".into()),
            ("Server", "es-bcn-01 · Barcelona, ES".into()),
            ("Protocol", "WireGuard".into()),
            ("Ping", "12 ms".into()),
            ("Sent", "4.8 MB".into()),
            ("Received", "31.2 MB".into()),
        ]);
    }
    Ok(0)
}

pub fn servers(_: &Args) -> Result<i32> {
    let rows: Vec<Vec<String>> = SERVERS
        .iter()
        .map(|(id, place, load, ping)| {
            vec![
                id.to_string(),
                place.to_string(),
                format!("{load}%"),
                format!("{ping} ms"),
            ]
        })
        .collect();
    ui::table(&["Server", "Place", "Load", "Ping"], &rows);
    Ok(0)
}

pub fn about(_: &Args) -> Result<i32> {
    ui::wordmark_with_icon("A fake VPN, themed with Ultraviolet");
    ui::blank();
    ui::kv(&[
        ("Version", env!("CARGO_PKG_VERSION").into()),
        (
            "Built with",
            format!("paloma {}", env!("CARGO_PKG_VERSION")),
        ),
        ("Docs", ui::link("https://github.com/mvilark/paloma")),
    ]);
    Ok(0)
}

pub fn is_cancel(err: &Error) -> bool {
    let Error::IO(io) = err;
    io.kind() == std::io::ErrorKind::Interrupted
}
