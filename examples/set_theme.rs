use paloma::{theme::ColorfulTheme, Confirm, Input};

fn main() {
    // Set the theme once; every prompt below uses it.
    paloma::set_theme(ColorfulTheme);

    let name: String = Input::new().prompt("Your name").interact().unwrap();

    // `.theme(&t)` still overrides the app theme for a single prompt.
    let ok = Confirm::new()
        .theme(&paloma::theme::SimpleTheme)
        .prompt(format!("Is {name} correct?"))
        .interact()
        .unwrap();
    println!("{}", if ok { "great" } else { "ok" });
}
