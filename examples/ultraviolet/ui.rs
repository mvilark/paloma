//! Output that is not a paloma prompt: sections, tasks, progress, outro box, tables, wordmark.
use std::{
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use console::{measure_text_width, Term};

use crate::theme::{cfg, Tok};

#[derive(Clone, Copy)]
pub enum Kind {
    Ok,
    Warn,
    Err,
    Info,
}

fn glyph(kind: Kind) -> String {
    let c = cfg();
    match kind {
        Kind::Ok => c.pal.e(Tok::Success, c.g.ok),
        Kind::Warn => c.pal.e(Tok::Warning, c.g.warn),
        Kind::Err => c.pal.e(Tok::Error, c.g.err),
        Kind::Info => c.pal.e(Tok::Accent, c.g.info),
    }
}

fn eprint_line(line: impl std::fmt::Display) {
    let _ = writeln!(Term::stderr(), "{line}");
}

pub fn blank() {
    eprint_line("");
}

/// `# title`
pub fn section(title: &str) {
    let c = cfg();
    eprint_line(format!(
        "{} {}",
        c.pal.e(Tok::Accent, "#"),
        c.pal.style(Tok::Fg).bold().for_stderr().apply_to(title)
    ));
}

/// A status message: only the glyph carries the state.
pub fn log(kind: Kind, msg: &str) {
    eprint_line(format!("{} {}", glyph(kind), cfg().pal.e(Tok::Fg, msg)));
}

/// `hint  <command to paste>`
pub fn hint(cmd: &str) {
    let c = cfg();
    eprint_line(format!(
        "  {}  {}",
        c.pal
            .style(Tok::Subtle)
            .italic()
            .for_stderr()
            .apply_to("Hint"),
        c.pal.e(Tok::Strong, cmd)
    ));
}

/// The collapsed record printed when the flow is cancelled.
pub fn cancelled() {
    let c = cfg();
    let _ = Term::stderr().show_cursor();
    eprint_line(format!(
        "{} … {} {}",
        c.pal.e(Tok::Subtle, c.g.answered),
        c.pal.e(Tok::Subtle, "·"),
        c.pal
            .style(Tok::Subtle)
            .italic()
            .for_stderr()
            .apply_to("Cancelled")
    ));
}

/// Key/value rows on stdout: values align to the longest key + 3.
pub fn kv(rows: &[(&str, String)]) {
    let c = cfg();
    let width = rows
        .iter()
        .map(|(k, _)| measure_text_width(k))
        .max()
        .unwrap_or(0)
        + 3;
    for (k, v) in rows {
        println!(
            "{}{}{}",
            c.pal.o(Tok::Muted, k),
            " ".repeat(width - measure_text_width(k)),
            c.pal.o(Tok::Fg, v)
        );
    }
}

/// A table on stdout with a bold header row.
pub fn table(header: &[&str], rows: &[Vec<String>]) {
    let c = cfg();
    let widths: Vec<usize> = (0..header.len())
        .map(|i| {
            rows.iter()
                .map(|r| measure_text_width(&r[i]))
                .chain([measure_text_width(header[i])])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let pad = |s: &str, w: usize| format!("{s}{}", " ".repeat(w - measure_text_width(s)));
    let head: Vec<String> = header
        .iter()
        .zip(&widths)
        .map(|(h, w)| c.pal.style(Tok::Fg).bold().apply_to(pad(h, *w)).to_string())
        .collect();
    println!("{}", head.join("   "));
    for r in rows {
        let cells: Vec<String> = r
            .iter()
            .zip(&widths)
            .map(|(s, w)| c.pal.o(Tok::Fg, pad(s, *w)))
            .collect();
        println!("{}", cells.join("   "));
    }
}

/// An OSC 8 hyperlink, underlined, when the terminal can show it.
pub fn link(url: &str) -> String {
    let c = cfg();
    let text = c
        .pal
        .style(Tok::Accent)
        .underlined()
        .for_stderr()
        .apply_to(url);
    if Term::stderr().features().colors_supported() {
        format!("\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\")
    } else {
        url.to_string()
    }
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
        c.pal.gradient("Ultraviolet"),
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

/// A running task: braille spinner, elapsed time, nested `↳` lines.
pub struct Task {
    start: Instant,
    state: Arc<Mutex<()>>,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Task {
    pub fn start(running: &str) -> Task {
        let stop = Arc::new(AtomicBool::new(false));
        let state = Arc::new(Mutex::new(()));
        let start = Instant::now();
        let mut handle = None;
        if cfg().motion {
            let (stop, state, label) = (stop.clone(), state.clone(), running.to_string());
            let term = Term::stderr();
            let _ = term.hide_cursor();
            handle = Some(thread::spawn(move || {
                let c = cfg();
                let mut frame = 0;
                while !stop.load(Ordering::Relaxed) {
                    {
                        let _guard = state.lock().unwrap();
                        let secs = start.elapsed().as_secs_f32();
                        let _ = term.clear_line();
                        let _ = write!(
                            &term,
                            "{} {}  {}",
                            c.pal.e(Tok::Accent, c.g.spinner[frame % c.g.spinner.len()]),
                            c.pal.e(Tok::Muted, &label),
                            c.pal.e(Tok::Subtle, format!("{secs:.1}s"))
                        );
                    }
                    frame += 1;
                    sleep(80);
                }
            }));
        }
        Task {
            start,
            state,
            stop,
            handle,
        }
    }

    /// A nested `↳` line under the running task.
    pub fn step(&self, msg: &str) {
        let c = cfg();
        let _guard = self.state.lock().unwrap();
        let term = Term::stderr();
        if c.motion {
            let _ = term.clear_line();
        }
        let _ = writeln!(
            &term,
            "    {} {}",
            c.pal.e(Tok::Faint, c.g.nested),
            c.pal.e(Tok::Muted, msg)
        );
    }

    /// Stops the spinner and prints the result line with the elapsed time.
    pub fn finish(mut self, kind: Kind, done: &str) -> f32 {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
            let term = Term::stderr();
            let _ = term.clear_line();
            let _ = term.show_cursor();
        }
        let secs = self.start.elapsed().as_secs_f32();
        let c = cfg();
        eprint_line(format!(
            "{} {}  {}",
            glyph(kind),
            c.pal.e(Tok::Fg, done),
            c.pal.e(Tok::Subtle, format!("{secs:.1}s"))
        ));
        secs
    }
}

/// Runs fake work: `secs` of sleeping with `steps` printed along the way.
pub fn task(running: &str, steps: &[&str], done: &str, kind: Kind, secs: f32) -> f32 {
    let t = Task::start(running);
    let pause = if cfg().motion {
        (secs * 1000.0) as u64 / (steps.len() as u64 + 1)
    } else {
        0
    };
    for s in steps {
        sleep(pause);
        t.step(s);
    }
    sleep(pause);
    t.finish(kind, done)
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

/// The outro box: a title, rows with a status glyph and a right-aligned duration.
pub fn outro(title: &str, rows: &[(Kind, String, Option<f32>)]) {
    let c = cfg();
    let width = (Term::stderr().size().1 as usize)
        .saturating_sub(2)
        .clamp(40, 64);
    let inner = width - 2;
    let edge = |s: &str| c.pal.e(Tok::Faint, s);
    let head_fill = inner.saturating_sub(measure_text_width(title) + 3);
    eprint_line(format!(
        "{}{} {} {}{}",
        edge(c.g.box_tl),
        edge(c.g.box_h),
        c.pal.style(Tok::Strong).bold().for_stderr().apply_to(title),
        edge(&c.g.box_h.repeat(head_fill)),
        edge(c.g.box_tr)
    ));
    for (kind, text, secs) in rows {
        let left = format!(" {} {}", glyph(*kind), c.pal.e(Tok::Fg, text));
        let right = secs
            .map(|s| c.pal.e(Tok::Subtle, format!("{s:.1}s")))
            .unwrap_or_default();
        let gap = inner.saturating_sub(measure_text_width(&left) + measure_text_width(&right) + 1);
        eprint_line(format!(
            "{}{}{}{} {}",
            edge(c.g.box_v),
            left,
            " ".repeat(gap),
            right,
            edge(c.g.box_v)
        ));
    }
    eprint_line(format!(
        "{}{}{}",
        edge(c.g.box_bl),
        edge(&c.g.box_h.repeat(inner)),
        edge(c.g.box_br)
    ));
}
