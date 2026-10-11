use paloma::{theme::ColorfulTheme, KeyValue, Message, Panel, PanelRow, Status, Table};

fn main() -> paloma::Result {
    paloma::set_theme(ColorfulTheme::default());

    paloma::section("Status").print()?;
    KeyValue::new()
        .row("State", "Connected")
        .row("Server", "es-bcn-01 · Barcelona, ES")
        .row(
            "Docs",
            paloma::text::link("https://github.com/mvilark/paloma").to_string(),
        )
        .print()?;

    paloma::section("Servers").print()?;
    Table::new(["Server", "Place", "Load"])
        .row(["us-nyc-01", "New York, US", "42%"])
        .row(["es-bcn-01", "Barcelona, ES", "23%"])
        .print()?;

    eprintln!();
    Panel::new("Setup complete")
        .row(PanelRow::from(Message::success("Signed in as ada@example.com")).trailing("1.8s"))
        .row(PanelRow::new("Kill switch is off").status(Status::Warning))
        .row("Connect on startup: On")
        .print()?;

    eprintln!(
        "{}",
        paloma::text::gradient("paloma", (213, 198, 255), (109, 74, 255))
    );
    Ok(())
}
