//! Golden transcripts of the Loam example's non-interactive commands.
//!
//! Loam draws nothing itself: its look is a token theme and every line is a paloma
//! component, so these transcripts pin how paloma draws a whole app. Each environment
//! gets one snapshot holding every command's exit code, stdout and stderr.
#![cfg(all(
    unix,
    feature = "fuzzy-select",
    feature = "password",
    feature = "history",
    feature = "completion",
    feature = "editor"
))]

mod common;

use std::{path::PathBuf, process::Command, sync::OnceLock};

const COMMANDS: &[&[&str]] = &[
    &["--yes"],
    &["init", "--yes"],
    &["init", "--yes", "--name", "the orchard", "--zone", "cool"],
    &["init", "--yes", "--name", "the allotment"],
    &["init", "--yes", "--zone", "polar"],
    &["init", "--yes", "--zone", "nowhere"],
    &["plant", "--yes"],
    &["water", "--yes"],
    &["harvest", "--yes"],
    &["journal", "--yes"],
    &["journal", "--yes", "--long"],
    &["status"],
    &["status", "--json"],
    &["almanac"],
    &["seeds"],
    &["nope"],
    &["status", "--nope"],
    &["help"],
    &["init", "--yes", "--help"],
];

/// Builds the example once and returns its path.
fn example() -> &'static PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let status = Command::new(cargo)
            .args(["build", "--quiet", "--example", "loam", "--all-features"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .expect("cargo runs");
        assert!(status.success(), "the loam example builds");
        // Integration tests live in `<target>/<profile>/deps/`; examples one level up.
        let exe = std::env::current_exe().unwrap();
        exe.parent()
            .and_then(|deps| deps.parent())
            .unwrap()
            .join("examples/loam")
    })
}

/// Runs every command with only `env` set (plus `CI=1`, so there is no motion and every
/// duration is `0.0s`).
fn transcript(env: &[(&str, &str)], extra_args: &[&str]) -> String {
    let mut out = String::new();
    for args in COMMANDS {
        let output = Command::new(example())
            .args(*args)
            .args(extra_args)
            .env_clear()
            .env("CI", "1")
            .envs(env.iter().copied())
            .output()
            .expect("the example runs");
        out += &format!(
            "$ loam {}\n--- exit {}\n--- stdout\n{}--- stderr\n{}\n",
            [*args, extra_args].concat().join(" "),
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
    out
}

#[test]
fn night_truecolor() {
    let env = [
        ("TERM", "xterm-256color"),
        ("COLORTERM", "truecolor"),
        ("CLICOLOR_FORCE", "1"),
    ];
    common::assert_snapshot("loam-night-truecolor", &transcript(&env, &[]));
}

#[test]
fn day_256() {
    let env = [
        ("TERM", "xterm-256color"),
        ("LOAM_THEME", "day"),
        ("CLICOLOR_FORCE", "1"),
    ];
    common::assert_snapshot("loam-day-256", &transcript(&env, &[]));
}

#[test]
fn night_16() {
    let env = [("TERM", "xterm"), ("CLICOLOR_FORCE", "1")];
    common::assert_snapshot("loam-night-16", &transcript(&env, &[]));
}

#[test]
fn no_color() {
    let env = [("TERM", "xterm-256color"), ("NO_COLOR", "1")];
    common::assert_snapshot("loam-no-color", &transcript(&env, &[]));
}

#[test]
fn ascii() {
    let env = [
        ("TERM", "xterm-256color"),
        ("COLORTERM", "truecolor"),
        ("CLICOLOR_FORCE", "1"),
    ];
    common::assert_snapshot("loam-ascii", &transcript(&env, &["--ascii"]));
}

/// `--plain` draws the same commands with `SimpleTheme`: the app has no look of its own,
/// and everything still reads well.
#[test]
fn plain() {
    let env = [("TERM", "xterm-256color"), ("NO_COLOR", "1")];
    common::assert_snapshot("loam-plain", &transcript(&env, &["--plain"]));
}
