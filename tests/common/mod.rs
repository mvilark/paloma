//! Golden-file helper shared by the integration tests.
//!
//! `assert_snapshot("name", actual)` compares `actual` with `tests/snapshots/name.txt`.
//! Run with `UPDATE_SNAPSHOTS=1` to write the files instead, then review the diff.
use std::{fs, path::PathBuf};

/// Escape characters are shown as `␛` so snapshots stay readable in diffs.
pub fn visible(text: &str) -> String {
    text.replace('\x1b', "␛")
}

pub fn assert_snapshot(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(format!("{name}.txt"));
    let actual = visible(actual);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing snapshot {}; run with UPDATE_SNAPSHOTS=1",
            path.display()
        )
    });
    if expected != actual {
        let line = expected
            .lines()
            .zip(actual.lines())
            .position(|(e, a)| e != a)
            .unwrap_or_else(|| expected.lines().count().min(actual.lines().count()));
        panic!(
            "snapshot {name} differs from line {}:\n  expected: {:?}\n  actual:   {:?}\n\
             Run with UPDATE_SNAPSHOTS=1 to accept the new output.",
            line + 1,
            expected.lines().nth(line).unwrap_or("<end>"),
            actual.lines().nth(line).unwrap_or("<end>"),
        );
    }
}
