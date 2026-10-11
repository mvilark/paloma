use std::{thread::sleep, time::Duration};

use paloma::{theme::ColorfulTheme, Confirm};

fn work(millis: u64) {
    sleep(Duration::from_millis(millis));
}

fn main() -> paloma::Result {
    paloma::set_theme(ColorfulTheme::default());

    // paloma installs no signal handler: give the terminal back on Ctrl+C.
    ctrlc::set_handler(|| {
        paloma::terminal::restore();
        std::process::exit(130);
    })
    .expect("could not set the Ctrl+C handler");

    // A spinner with steps printed above it, ending in a success line.
    let task = paloma::task("Installing dependencies...").start()?;
    work(1200);
    task.step("Resolved 214 packages")?;
    work(1200);
    task.step("Downloaded 12 archives")?;
    work(1200);
    task.success("Installed dependencies")?;

    // A task can end with a warning, and the label can change while it runs.
    let task = paloma::task("Building...").start()?;
    work(1200);
    task.label("Linking...")?;
    work(1200);
    task.warning("Built with 2 warnings")?;

    // A prompt shown while a task runs pauses the spinner; it comes back after the
    // answer. Without a terminal there is nobody to ask, so the answer is no.
    let task = paloma::task("Publishing 1.2.0...").start()?;
    work(1200);
    let replace = Confirm::new()
        .prompt("Release 1.2.0 exists. Replace it?")
        .default(false)
        .interact()
        .unwrap_or(false);
    work(1200);
    if replace {
        task.success("Replaced release 1.2.0")?;
    } else {
        task.fail("Release 1.2.0 already exists")?;
    }
    Ok(())
}
