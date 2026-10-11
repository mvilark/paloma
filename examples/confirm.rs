use paloma::{theme::ColorfulTheme, Confirm};

fn main() {
    if Confirm::new()
        .theme(&ColorfulTheme::default())
        .prompt("Do you want to continue?")
        .interact()
        .unwrap()
    {
        println!("Looks like you want to continue");
    } else {
        println!("nevermind then :(");
    }

    if Confirm::new()
        .theme(&ColorfulTheme::default())
        .prompt("Do you really want to continue?")
        .default(true)
        .interact()
        .unwrap()
    {
        println!("Looks like you want to continue");
    } else {
        println!("nevermind then :(");
    }

    if Confirm::new()
        .theme(&ColorfulTheme::default())
        .prompt("Do you really really want to continue?")
        .default(true)
        .show_default(false)
        .on_shortcut(false)
        .interact()
        .unwrap()
    {
        println!("Looks like you want to continue");
    } else {
        println!("nevermind then :(");
    }

    if Confirm::new()
        .theme(&ColorfulTheme::default())
        .prompt("Do you really really really want to continue?")
        .on_shortcut(false)
        .interact()
        .unwrap()
    {
        println!("Looks like you want to continue");
    } else {
        println!("nevermind then :(");
    }

    match Confirm::new()
        .theme(&ColorfulTheme::default())
        .prompt("Do you really really really really want to continue?")
        .interact_opt()
        .unwrap()
    {
        Some(true) => println!("Looks like you want to continue"),
        Some(false) => println!("nevermind then :("),
        None => println!("Ok, we can start over later"),
    }

    match Confirm::new()
        .theme(&ColorfulTheme::default())
        .prompt("Do you really really really really really want to continue?")
        .default(true)
        .on_shortcut(false)
        .interact_opt()
        .unwrap()
    {
        Some(true) => println!("Looks like you want to continue"),
        Some(false) => println!("nevermind then :("),
        None => println!("Ok, we can start over later"),
    }
}
