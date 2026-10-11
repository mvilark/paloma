//! The message row under prompts: a description, live messages while typing, and
//! validation errors that keep the input.
use paloma::{theme::ColorfulTheme, Input, Message, MultiSelect, Password};

fn main() -> paloma::Result<()> {
    paloma::set_theme(ColorfulTheme);

    // A description while nothing else applies; a refused name shows its error in the
    // row and keeps what was typed.
    let name: String = Input::new()
        .prompt("Username")
        .description("3 to 16 letters or digits")
        .placeholder("ada")
        .validate(|name: &String| -> Result<(), &str> {
            let len = name.chars().count();
            if !(3..=16).contains(&len) {
                Err("Use 3 to 16 characters")
            } else if !name.chars().all(char::is_alphanumeric) {
                Err("Use only letters and digits")
            } else {
                Ok(())
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
