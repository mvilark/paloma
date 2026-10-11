//! A sign-up flow: the step counter, Esc to go back, `back_to` with a message, a
//! conditional step, `keep_last` and a collapsed summary at the end.
use std::{thread, time::Duration};

use paloma::{flow, theme::ColorfulTheme, Confirm, Done, Flow, Input, Message, Password};

#[derive(Default)]
struct SignUp {
    email: String,
    password: String,
    tries: u32,
    custom_server: bool,
    port: u16,
}

fn main() -> paloma::Result {
    paloma::set_theme(ColorfulTheme);
    // `Ctrl+C`, or `Esc` on the first step, exits with code 130 (the default); this
    // message is printed on its own line first.
    paloma::set_options(
        paloma::Options::new().general(|g| g.cancel_message(Message::warning("Sign-up cancelled"))),
    );

    let state = Flow::new(SignUp {
        port: 443,
        ..SignUp::default()
    })
    .title("Sign up")
    // Only the last two answered steps stay on screen while the flow runs, and one
    // summary line once it is done.
    .keep_last(2)
    .done(Done::Collapse)
    .step("email", |s| {
        // Going back keeps the earlier answer: the input opens with it.
        s.email = Input::new()
            .prompt("Email")
            .prefill(s.email.clone())
            .validate(|email: &String| {
                if email.contains('@') {
                    Ok(())
                } else {
                    Err("Enter an address like ada@example.com")
                }
            })
            .interact()?;
        Ok(())
    })
    .step("password", |s| {
        s.password = Password::new().prompt("Password").interact()?;
        Ok(())
    })
    // A step does not have to ask anything: here it signs in, with a spinner.
    .step("sign in", |s| {
        let task = paloma::task("Signing in...").start()?;
        thread::sleep(Duration::from_millis(500));
        s.tries += 1;
        if s.tries == 1 {
            // The first try always fails, to show `back_to`.
            task.clear()?;
            return flow::back_to("email", Message::error("Wrong email or password"));
        }
        task.success(format!("Signed in as {}", s.email))?;
        Ok(())
    })
    .step("server", |s| {
        s.custom_server = Confirm::new()
            .prompt("Use a custom server?")
            .default(s.custom_server)
            .interact()?;
        Ok(())
    })
    // Counted in the total (`05`) only once the confirm above said yes.
    .step_if(
        |s| s.custom_server,
        "port",
        |s| {
            s.port = Input::new().prompt("Port").default(s.port).interact()?;
            Ok(())
        },
    )
    .run()?;

    let server = if state.custom_server {
        format!("port {}", state.port)
    } else {
        "the default server".to_string()
    };
    paloma::success(format!("Signed up as {} on {server}", state.email)).print()
}
