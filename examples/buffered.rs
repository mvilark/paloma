use console::Term;
use paloma::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select, Sort};

fn main() {
    let items = &[
        "Ice Cream",
        "Vanilla Cupcake",
        "Chocolate Muffin",
        "A Pile of sweet, sweet mustard",
    ];
    let term = Term::buffered_stderr();
    let theme = ColorfulTheme::default();

    println!("All the following controls are run in a buffered terminal");
    Confirm::new()
        .theme(&theme)
        .prompt("Do you want to continue?")
        .interact_on(&term)
        .unwrap();

    let _: String = Input::new()
        .theme(&theme)
        .prompt("Your name")
        .interact_on(&term)
        .unwrap();

    Select::new()
        .theme(&theme)
        .prompt("Pick an item")
        .items(items)
        .interact_on(&term)
        .unwrap();

    MultiSelect::new()
        .theme(&theme)
        .prompt("Pick some items")
        .items(items)
        .interact_on(&term)
        .unwrap();

    Sort::new()
        .theme(&theme)
        .prompt("Order these items")
        .items(items)
        .interact_on(&term)
        .unwrap();
}
