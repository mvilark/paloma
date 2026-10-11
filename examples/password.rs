use paloma::{theme::ColorfulTheme, Password};

fn main() {
    let password = Password::new()
        .theme(&ColorfulTheme)
        .prompt("Password")
        .confirmation("Repeat password", "Error: the passwords don't match.")
        // .mask_char('*') // uncomment to mask the password with a given character
        .validate(|input: &String| -> Result<(), &str> {
            if input.chars().count() > 3 {
                Ok(())
            } else {
                Err("Password must be longer than 3")
            }
        })
        .interact()
        .unwrap();

    println!(
        "Your password is {} characters long",
        password.chars().count()
    );
}
