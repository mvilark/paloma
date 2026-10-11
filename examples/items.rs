//! Items with a description and a disabled state, and the line of keys under a prompt.
use paloma::{theme::ColorfulTheme, Item, MultiSelect, Select};

fn main() -> paloma::Result<()> {
    paloma::set_theme(ColorfulTheme);

    // A disabled item is shown but the highlight skips it, and it cannot be chosen.
    let plans = [
        Item::new("Free").description("1 device"),
        Item::new("Team").description("10 devices"),
        Item::new("Enterprise")
            .description("talk to sales")
            .disabled(true),
        Item::new("Hobby").description("3 devices"),
    ];
    let plan = Select::new()
        .prompt("Plan")
        .items(plans.clone())
        .default(0)
        .legend(true)
        .interact()?;
    println!("Plan: {plan}");

    // `Esc` goes back (the prompt leaves nothing), `Ctrl+C` returns `Error::Cancelled`.
    // A disabled item keeps its state: this one is checked and stays checked.
    let features = MultiSelect::new()
        .prompt("Features")
        .item_checked(
            Item::new("Updates").description("always on").disabled(true),
            true,
        )
        .item(Item::new("Backups").description("daily"))
        .item(
            Item::new("Audit log")
                .description("coming soon")
                .disabled(true),
        )
        .item("Support")
        .legend(true)
        .interact_opt()?;
    match features {
        Some(features) => println!("Features: {features:?}"),
        None => println!("Went back"),
    }
    Ok(())
}
