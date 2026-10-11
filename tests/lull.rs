//! Golden transcripts of the Lull example's non-interactive commands.
//!
//! Lull's look is two themes that follow the user's sleep schedule, so the same command
//! is drawn by the day or the night theme depending on `LULL_NOW`. Each snapshot holds
//! the exit code, stdout and stderr of every command it lists.
#![cfg(all(unix, feature = "fuzzy-select"))]

mod common;

use std::{path::PathBuf, process::Command, sync::OnceLock};

const FILE: &str = "tests/fixtures/lull.json";
const BROKEN: &str = "tests/fixtures/lull-broken.json";
const MISSING: &str = "tests/fixtures/lull-missing.json";

const ALL_COLORS: &[(&str, &str)] = &[("TERM", "xterm-256color"), ("CLICOLOR_FORCE", "1")];

/// A command to run: `LULL_NOW`, `LULL_FILE` and the arguments.
type Case = (&'static str, &'static str, &'static [&'static str]);

/// Builds the example once and returns its path.
fn example() -> &'static PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let status = Command::new(cargo)
            .args([
                "build",
                "--quiet",
                "--example",
                "lull",
                "--features",
                "fuzzy-select",
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .expect("cargo runs");
        assert!(status.success(), "the lull example builds");
        // Integration tests live in `<target>/<profile>/deps/`; examples one level up.
        let exe = std::env::current_exe().unwrap();
        exe.parent()
            .and_then(|deps| deps.parent())
            .unwrap()
            .join("examples/lull")
    })
}

/// Runs every case with only `env` set (plus `TZ=UTC`).
fn transcript(env: &[(&str, &str)], cases: &[Case]) -> String {
    let mut out = String::new();
    for (now, file, args) in cases {
        let output = Command::new(example())
            .args(*args)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env_clear()
            .env("TZ", "UTC")
            .env("LULL_NOW", now)
            .env("LULL_FILE", file)
            .envs(env.iter().copied())
            .output()
            .expect("the example runs");
        out += &format!(
            "$ LULL_NOW={now} LULL_FILE={file} lull {}\n--- exit {}\n--- stdout\n{}--- stderr\n{}\n",
            args.join(" "),
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
    out
}

#[test]
fn day() {
    let cases: &[Case] = &[
        ("2026-10-11T14:00", FILE, &["today"]),
        ("2026-10-11T14:00", FILE, &["week"]),
        ("2026-10-11T14:00", FILE, &["nope"]),
        ("2026-10-11T14:00", MISSING, &["today"]),
    ];
    common::assert_snapshot("lull-day", &transcript(ALL_COLORS, cases));
}

#[test]
fn wind_down() {
    let cases: &[Case] = &[("2026-10-11T22:20", FILE, &["today"])];
    common::assert_snapshot("lull-wind-down", &transcript(ALL_COLORS, cases));
}

#[test]
fn night() {
    let cases: &[Case] = &[
        ("2026-10-12T00:40", FILE, &["today"]),
        ("2026-10-12T00:40", FILE, &["week"]),
    ];
    common::assert_snapshot("lull-night", &transcript(ALL_COLORS, cases));
}

/// The hint to log last night shows only while last night is not logged.
#[test]
fn morning() {
    let cases: &[Case] = &[
        ("2026-10-11T07:30", FILE, &["today"]),
        ("2026-10-12T07:30", FILE, &["today"]),
    ];
    common::assert_snapshot("lull-morning", &transcript(ALL_COLORS, cases));
}

#[test]
fn basic_colors() {
    let env = [("TERM", "xterm"), ("CLICOLOR_FORCE", "1")];
    let cases: &[Case] = &[
        ("2026-10-11T14:00", FILE, &["today"]),
        ("2026-10-12T00:40", FILE, &["today"]),
    ];
    common::assert_snapshot("lull-16", &transcript(&env, cases));
}

#[test]
fn no_color() {
    let env = [("TERM", "xterm-256color"), ("NO_COLOR", "1")];
    let cases: &[Case] = &[("2026-10-12T00:40", FILE, &["today"])];
    common::assert_snapshot("lull-no-color", &transcript(&env, cases));
}

/// A terminal without Unicode gets `SimpleTheme`.
#[test]
fn ascii() {
    let env = [("TERM", "linux"), ("CLICOLOR_FORCE", "1")];
    let cases: &[Case] = &[
        ("2026-10-12T00:40", FILE, &["today"]),
        ("2026-10-12T00:40", FILE, &["week"]),
    ];
    common::assert_snapshot("lull-ascii", &transcript(&env, cases));
}

/// A file that cannot be read is an error, never overwritten.
#[test]
fn broken() {
    let cases: &[Case] = &[("2026-10-11T14:00", BROKEN, &["today"])];
    common::assert_snapshot("lull-broken", &transcript(ALL_COLORS, cases));
}
