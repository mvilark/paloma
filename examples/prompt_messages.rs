//! The message row under prompts: a description, live messages while typing, slow
//! checks in the background, and validation errors that keep the input.
use std::{thread, time::Duration};

use paloma::{theme::ColorfulTheme, Input, Message, MultiSelect, Password};

/// Names a fake server already knows.
const TAKEN: [&str; 3] = ["admin", "root", "ada"];

fn main() -> paloma::Result<()> {
    paloma::set_theme(ColorfulTheme);

    // A description while nothing else applies; a refused name shows its error in the
    // row and keeps what was typed. Whether the name is free is a slow check (a fake
    // server call): it runs in the background once typing pauses for 300 ms, while
    // paloma keeps reading keys, and a result for a name that changed meanwhile is
    // dropped. The pending message shows while the check, or the validation on
    // submit, runs.
    let name: String = Input::new()
        .prompt("Username")
        .description("3 to 16 letters or digits")
        .placeholder("ada")
        .change_delay(300)
        .pending(Message::info("Checking the name…"))
        .on_change(|name: &str| {
            thread::sleep(Duration::from_millis(500));
            if name.is_empty() {
                None
            } else if TAKEN.contains(&name.to_lowercase().as_str()) {
                Some(Message::warning(format!("{name} is taken")))
            } else {
                Some(Message::success(format!("{name} is available")))
            }
        })
        .validate(|name: &String| -> Result<(), &str> {
            let len = name.chars().count();
            if !(3..=16).contains(&len) {
                Err("Use 3 to 16 characters")
            } else if !name.chars().all(char::is_alphanumeric) {
                Err("Use only letters and digits")
            } else {
                // The server has the last word.
                thread::sleep(Duration::from_millis(300));
                if TAKEN.contains(&name.to_lowercase().as_str()) {
                    Err("That name is taken")
                } else {
                    Ok(())
                }
            }
        })
        .interact()?;

    // The rules that still fail, recomputed on each key; each hint goes away once its
    // rule is met, and the description comes back when all pass.
    Password::new()
        .prompt("Password")
        .description("Choose a strong password")
        .mask(true)
        .on_change(|pw: &str| {
            let missing: Vec<&str> = [
                (pw.chars().count() < 8, "At least 8 characters"),
                (!pw.chars().any(|c| c.is_ascii_digit()), "One digit"),
                (!pw.chars().any(char::is_uppercase), "One uppercase letter"),
            ]
            .into_iter()
            .filter_map(|(fails, rule)| fails.then_some(rule))
            .collect();
            (!missing.is_empty()).then(|| {
                missing
                    .into_iter()
                    .fold(Message::error("Password too weak"), |msg, rule| {
                        msg.nest(Message::hint(rule))
                    })
            })
        })
        .validate(|pw: &String| -> Result<(), &str> {
            if pw.chars().count() >= 8 {
                Ok(())
            } else {
                Err("Password too weak")
            }
        })
        .interact()?;

    // List prompts validate too.
    let features = MultiSelect::new()
        .prompt("Features")
        .items(["Kill switch", "Split tunnelling", "Ad blocking"])
        .description(Message::hint("Space to check, Enter to confirm"))
        .validate(|checked: &[usize]| {
            if checked.is_empty() {
                Err("Pick at least one feature")
            } else {
                Ok(())
            }
        })
        .interact()?;

    println!("{name}: {} feature(s)", features.len());
    Ok(())
}
