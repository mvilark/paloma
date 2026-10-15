use std::{thread::sleep, time::Duration};

use paloma::{
    theme::{ColorfulTheme, Theme},
    Confirm,
};

fn work(millis: u64) {
    sleep(Duration::from_millis(millis));
}

fn main() -> paloma::Result {
    // The default look, plus a two-cell glint at the head of a running bar.
    paloma::set_theme(ColorfulTheme.restyle(|t| t.live(|l| l.progress(|p| p.head_width(2)))));

    // paloma installs no signal handler: give the terminal back on Ctrl+C.
    ctrlc::set_handler(|| {
        paloma::terminal::restore();
        std::process::exit(130);
    })
    .expect("could not set the Ctrl+C handler");

    // A download-like bar: 480 kilobytes in chunks of 8, ending in a success line that
    // replaces the bar. The bar eases toward each value and never moves backwards.
    let bar = paloma::progress("Downloading paloma-1.2.0.tar.gz...", 480).start()?;
    for _ in 0..60 {
        work(35);
        bar.inc(8)?;
    }
    bar.success("Downloaded paloma-1.2.0.tar.gz")?;

    // A bar that stays on screen as a lasting line, at its value, when the work is done.
    let bar = paloma::progress("Compiling 40 crates", 40).start()?;
    for crate_number in 1..=40 {
        work(50);
        if crate_number == 20 {
            bar.label("Compiling 40 crates (halfway)")?;
        }
        bar.set(crate_number)?;
    }
    bar.keep()?;

    // A prompt shown while a bar runs pauses it; the bar comes back after the answer.
    // Without a terminal there is nobody to ask, so the answer is no.
    let bar = paloma::progress("Uploading 3 files...", 3).start()?;
    work(900);
    bar.inc(1)?;
    work(900);
    let large = Confirm::new()
        .prompt("The next file is 2 GB. Upload it?")
        .default(false)
        .interact()
        .unwrap_or(false);
    if large {
        work(1200);
        bar.inc(2)?;
        work(600);
        bar.success("Uploaded 3 files")?;
    } else {
        bar.warning("Uploaded 1 file, skipped 2")?;
    }
    Ok(())
}
