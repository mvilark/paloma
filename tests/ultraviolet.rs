//! Golden transcripts of the Ultraviolet example's non-interactive commands.
//!
//! PAL-004 moves the example's drawing code into paloma; its output must not change.
//! Each environment gets one snapshot holding every command's exit code, stdout and stderr.
#![cfg(all(unix, feature = "fuzzy-select", feature = "password"))]

mod common;

use std::{path::PathBuf, process::Command, sync::OnceLock};

const COMMANDS: &[&[&str]] = &[
    &["setup", "--yes"],
    &["connect", "--yes"],
    &["connect", "--server", "xx-nowhere-01"],
    &["connect", "--server", "es-bcn-01", "--yes"],
    &["favorites", "--yes"],
    &["disconnect", "--yes"],
    &["status"],
    &["status", "--json"],
    &["servers"],
    &["about"],
    &["nope"],
];

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
                "ultraviolet",
                "--all-features",
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .expect("cargo runs");
        assert!(status.success(), "the ultraviolet example builds");
        // Integration tests live in `<target>/<profile>/deps/`; examples one level up.
        let exe = std::env::current_exe().unwrap();
        exe.parent()
            .and_then(|deps| deps.parent())
            .unwrap()
            .join("examples/ultraviolet")
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
            "$ ultraviolet {}\n--- exit {}\n--- stdout\n{}--- stderr\n{}\n",
            [*args, extra_args].concat().join(" "),
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
    out
}

#[test]
fn dark_truecolor() {
    let env = [
        ("TERM", "xterm-256color"),
        ("COLORTERM", "truecolor"),
        ("CLICOLOR_FORCE", "1"),
    ];
    common::assert_snapshot("ultraviolet-dark-truecolor", &transcript(&env, &[]));
}

#[test]
fn light_256() {
    let env = [
        ("TERM", "xterm-256color"),
        ("ULTRAVIOLET_THEME", "light"),
        ("CLICOLOR_FORCE", "1"),
    ];
    common::assert_snapshot("ultraviolet-light-256", &transcript(&env, &[]));
}

#[test]
fn dark_16() {
    let env = [("TERM", "xterm"), ("CLICOLOR_FORCE", "1")];
    common::assert_snapshot("ultraviolet-dark-16", &transcript(&env, &[]));
}

#[test]
fn no_color() {
    let env = [("TERM", "xterm-256color"), ("NO_COLOR", "1")];
    common::assert_snapshot("ultraviolet-no-color", &transcript(&env, &[]));
}

#[test]
fn ascii() {
    let env = [
        ("TERM", "xterm-256color"),
        ("COLORTERM", "truecolor"),
        ("CLICOLOR_FORCE", "1"),
    ];
    common::assert_snapshot("ultraviolet-ascii", &transcript(&env, &["--ascii"]));
}
