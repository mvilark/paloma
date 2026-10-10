use paloma::{theme::ColorfulTheme, Input};

fn main() {
    let input: String = Input::new()
        .theme(&ColorfulTheme::default())
        .label("Your name")
        .interact()
        .unwrap();

    println!("Hello {}!", input);

    let mail: String = Input::new()
        .theme(&ColorfulTheme::default())
        .label("Your email")
        .validate({
            let mut force = None;
            move |input: &String| -> Result<(), &str> {
                if input.contains('@') || (force.as_ref() == Some(input)) {
                    Ok(())
                } else {
                    force = Some(input.clone());
                    Err("This is not a mail address; type the same value again to force use")
                }
            }
        })
        .interact()
        .unwrap();

    println!("Email: {}", mail);

    let mail: String = Input::new()
        .theme(&ColorfulTheme::default())
        .label("Your planet")
        .default("Earth".to_string())
        .interact()
        .unwrap();

    println!("Planet: {}", mail);

    let mail: String = Input::new()
        .theme(&ColorfulTheme::default())
        .label("Your galaxy")
        .initial_text("Milky Way".to_string())
        .interact()
        .unwrap();

    println!("Galaxy: {}", mail);
}
