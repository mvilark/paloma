//! What paloma has no tool for yet: the Serrallonga icon and wordmark, the progress bar
//! (until PAL-011) and the fake-work pauses. Everything else is a paloma component.
use std::{io::Write, thread, time::Duration};

use console::{Style, Term};
use paloma::text::gradient;

use crate::theme::{cfg, Tok};

fn eprint_line(line: impl std::fmt::Display) {
    let _ = writeln!(Term::stderr(), "{line}");
}

/// The app name in `title`, with a gradient from `wordmark-from` to `wordmark-to` where
/// 24-bit colour is shown and `accent-strong` elsewhere.
fn wordmark() -> String {
    let c = cfg();
    let (from, to) = c.pal.wordmark();
    gradient("Ultraviolet", from, to)
        .style(Style::new().bold())
        .fallback(c.pal.style(Tok::Strong).bold())
        .to_string()
}

/// The wordmark with the Serrallonga icon beside it (our own 8-line drawing).
pub fn wordmark_with_icon(tagline: &str) {
    let c = cfg();
    let icon: [&str; 8] = if c.g.ascii {
        [
            r"   ___   ",
            r"  /###\  ",
            r" (=o o=) ",
            r"  \ ~ /  ",
            r" /|x x|\ ",
            r"/ |x x| \",
            r"  |   |  ",
            r"  d   b  ",
        ]
    } else {
        [
            "  ▄████▄  ",
            " ▐██████▌ ",
            " ▐▀◉  ◉▀▌ ",
            "  ▝▄▂▂▄▘  ",
            " ╱▓▚▞▚▞▓╲ ",
            "╱ ▓▞▚▞▚▓ ╲",
            "  ▐▌  ▐▌  ",
            "  ▀▀  ▀▀  ",
        ]
    };
    let side = [
        String::new(),
        String::new(),
        String::new(),
        wordmark(),
        c.pal.e(Tok::Muted, tagline),
        String::new(),
        String::new(),
        String::new(),
    ];
    for (art, text) in icon.iter().zip(side) {
        eprint_line(format!("{}  {}", c.pal.e(Tok::Accent, art), text));
    }
}

fn sleep(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

/// Fake work: sleeps `secs` in total, split into `parts` pauses, when there is motion.
/// Without motion nothing waits, so every duration reads `0.0s`.
pub fn work(secs: f32, parts: usize) {
    if cfg().motion {
        sleep((secs * 1000.0) as u64 / parts.max(1) as u64);
    }
}

/// A progress bar eased toward 100%, with a two-cell glint at the head.
pub fn progress(label: &str) {
    let c = cfg();
    let width = 32usize;
    let term = Term::stderr();
    if !c.motion {
        eprint_line(format!(
            "{} {}  {}",
            c.pal.e(Tok::Accent, c.g.bar.repeat(width)),
            c.pal.e(Tok::Muted, label),
            c.pal.e(Tok::Subtle, "100%")
        ));
        return;
    }
    let _ = term.hide_cursor();
    for cell in 0..=width {
        let glint = cell.saturating_sub(2);
        let fill = format!(
            "{}{}",
            c.pal.e(Tok::Accent, c.g.bar.repeat(glint)),
            c.pal.e(Tok::Strong, c.g.bar.repeat(cell - glint))
        );
        let track = c.pal.e(Tok::Faint, c.g.bar.repeat(width - cell));
        let _ = term.clear_line();
        let _ = write!(
            &term,
            "{fill}{track} {}  {}",
            c.pal.e(Tok::Muted, label),
            c.pal.e(Tok::Subtle, format!("{}%", cell * 100 / width))
        );
        // Eased: fast at first, slower toward the end.
        sleep(45 + (cell as u64 * 40 / width as u64));
    }
    let _ = term.clear_line();
    let _ = term.show_cursor();
    eprint_line(format!(
        "{} {}  {}",
        c.pal.e(Tok::Accent, c.g.bar.repeat(width)),
        c.pal.e(Tok::Muted, label),
        c.pal.e(Tok::Subtle, "100%")
    ));
}
