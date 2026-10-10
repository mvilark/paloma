use paloma::{theme::ColorfulTheme, Confirm, Input};

fn main() {
    // Set the theme once; every prompt below uses it.
    paloma::set_theme(ColorfulTheme::default());

    let name: String = Input::new()
        .with_prompt("Your name")
        .interact_text()
        .unwrap();

    // `with_theme` still overrides the app theme for a single prompt.
    let ok = Confirm::with_theme(&paloma::theme::SimpleTheme)
        .with_prompt(format!("Is {name} correct?"))
        .interact()
        .unwrap();
    println!("{}", if ok { "great" } else { "ok" });
}
