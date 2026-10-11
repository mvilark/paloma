use paloma::{
    console::Style,
    terminal::Stream,
    theme::{ColorfulTheme, Theme},
    Message, Options,
};

fn main() -> paloma::Result {
    // Out of the box: plain words, like rustc and cargo.
    paloma::info("Resolving dependencies").print()?;
    paloma::error("Port 3000 is already in use")
        .nest(Message::hint("Stop the other server or pick another port"))
        .print()?;

    // A built-in theme, restyled without implementing `Theme`.
    paloma::set_theme(ColorfulTheme::default().restyle(|t| {
        t.palette(|p| p.info(Style::new().magenta()))
            .messages(|m| m.warning(|w| w.word(Some("careful:"))))
    }));
    paloma::warning("The cache is 3 days old").print()?;
    paloma::success("Installed 214 packages")
        .nest(Message::description("in 1.8s"))
        .print()?;

    // Where messages go is an option: results on stdout, everything else on stderr.
    paloma::set_options(Options::new().success(|s| s.stream(Stream::Stdout)));
    paloma::success("Wrote dist/app.tar.gz").print()?;
    Ok(())
}
