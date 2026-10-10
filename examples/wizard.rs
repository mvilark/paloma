use std::error::Error;
use std::net::IpAddr;

use console::Style;
use paloma::{theme::ColorfulTheme, Confirm, Input, Select};

#[derive(Debug)]
#[allow(dead_code)]
struct Config {
    interface: IpAddr,
    hostname: String,
    use_acme: bool,
    private_key: Option<String>,
    cert: Option<String>,
}

fn init_config() -> Result<Option<Config>, Box<dyn Error>> {
    let theme = ColorfulTheme {
        values_style: Style::new().yellow().dim(),
        ..ColorfulTheme::default()
    };
    println!("Welcome to the setup wizard");

    if !Confirm::new()
        .theme(&theme)
        .prompt("Do you want to continue?")
        .interact()?
    {
        return Ok(None);
    }

    let interface = Input::new()
        .theme(&theme)
        .prompt("Interface")
        .default("127.0.0.1".parse().unwrap())
        .interact()?;

    let hostname = Input::new().theme(&theme).prompt("Hostname").interact()?;

    let tls = Select::new()
        .theme(&theme)
        .prompt("Configure TLS")
        .default(0)
        .item("automatic with ACME")
        .item("manual")
        .item("no")
        .interact()?;

    let (private_key, cert, use_acme) = match tls {
        0 => (Some("acme.pkey".into()), Some("acme.cert".into()), true),
        1 => (
            Some(
                Input::new()
                    .theme(&theme)
                    .prompt("  Path to private key")
                    .interact()?,
            ),
            Some(
                Input::new()
                    .theme(&theme)
                    .prompt("  Path to certificate")
                    .interact()?,
            ),
            false,
        ),
        _ => (None, None, false),
    };

    Ok(Some(Config {
        hostname,
        interface,
        private_key,
        cert,
        use_acme,
    }))
}

fn main() {
    match init_config() {
        Ok(None) => println!("Aborted."),
        Ok(Some(config)) => println!("{:#?}", config),
        Err(err) => println!("error: {}", err),
    }
}
