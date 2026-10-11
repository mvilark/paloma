//! Flows driven without a terminal: every prompt answers with a preset or in unattended
//! mode, on a terminal that records what is written. A flow and `set_options` are
//! global, so everything is one test function.
#![cfg(unix)]
mod common;

use std::{cell::RefCell, io::ErrorKind};

use paloma::{flow, theme::SimpleTheme, Confirm, Done, Error, Flow, Input, Message, Options};

use common::capture::term;

#[derive(Default, Debug)]
struct State {
    name: String,
    custom: bool,
    port: u16,
    runs: Vec<&'static str>,
}

#[test]
fn flows_run_their_steps_over_the_state() {
    let theme = SimpleTheme;
    let (term, out) = term();
    let name = |preset: &str| {
        Input::<String>::new()
            .theme(&theme)
            .prompt("Name")
            .preset(Some(preset.to_string()))
    };

    // Steps run in order over the state; a conditional step runs only while its
    // condition holds.
    for custom in [false, true] {
        let state = Flow::new(State::default())
            .step("name", |s| {
                s.runs.push("name");
                s.name = name("Ada").interact_on(&term)?;
                Ok(())
            })
            .step("custom", |s| {
                s.runs.push("custom");
                s.custom = Confirm::new()
                    .theme(&theme)
                    .prompt("Custom?")
                    .preset(Some(custom))
                    .interact_on(&term)?;
                Ok(())
            })
            .step_if(
                |s| s.custom,
                "port",
                |s| {
                    s.runs.push("port");
                    s.port = 8080;
                    Ok(())
                },
            )
            .run()
            .unwrap();
        assert_eq!(state.name, "Ada");
        let expected: &[&str] = if custom {
            &["name", "custom", "port"]
        } else {
            &["name", "custom"]
        };
        assert_eq!(state.runs, expected);
        assert_eq!(state.port, if custom { 8080 } else { 0 });
        assert_eq!(
            out.take(),
            format!("Name: Ada\nCustom? {}\n", if custom { "yes" } else { "no" })
        );
    }

    // `step_done` applies to prompts that set no `done` of their own.
    Flow::new(())
        .step_done(Done::Erase)
        .step("a", |_| name("Ada").interact_on(&term).map(drop))
        .step("b", |_| {
            name("Bob")
                .done(Done::Collapse)
                .interact_on(&term)
                .map(drop)
        })
        .run()
        .unwrap();
    assert_eq!(out.take(), "Name: Bob\n");
    // Outside a flow it no longer applies.
    name("Cy").interact_on(&term).unwrap();
    assert_eq!(out.take(), "Name: Cy\n");

    // `Options::flows` sets it app-wide; the flow's own setter wins.
    paloma::set_options(Options::new().flows(|f| f.step_done(Done::Erase)));
    Flow::new(())
        .step("a", |_| name("Ada").interact_on(&term).map(drop))
        .run()
        .unwrap();
    assert_eq!(out.take(), "");
    Flow::new(())
        .step_done(Done::Collapse)
        .step("a", |_| name("Ada").interact_on(&term).map(drop))
        .run()
        .unwrap();
    assert_eq!(out.take(), "Name: Ada\n");
    paloma::set_options(Options::new());

    // `back_to` runs the named step again, then the steps after it.
    let tries = RefCell::new(0);
    let state = Flow::new(State::default())
        .step("name", |s| {
            s.runs.push("name");
            Ok(())
        })
        .step("check", |s| {
            s.runs.push("check");
            *tries.borrow_mut() += 1;
            if *tries.borrow() == 1 {
                return flow::back_to("name", Message::error("Taken"));
            }
            Ok(())
        })
        .step("done", |s| {
            s.runs.push("done");
            Ok(())
        })
        .run()
        .unwrap();
    assert_eq!(state.runs, ["name", "check", "name", "check", "done"]);

    // The current step's own name runs it again.
    let tries = RefCell::new(0);
    Flow::new(())
        .step("retry", |_| {
            *tries.borrow_mut() += 1;
            if *tries.borrow() < 3 {
                return flow::back_to("retry", None);
            }
            Ok(())
        })
        .run()
        .unwrap();
    assert_eq!(*tries.borrow(), 3);

    // An unknown or later step is an error naming it.
    for target in ["nope", "later"] {
        let err = Flow::new(())
            .step("first", |_| Ok(()))
            .step("second", |_| flow::back_to(target, None))
            .step("later", |_| Ok(()))
            .run()
            .unwrap_err();
        let err = std::io::Error::from(err);
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert!(err.to_string().contains(target), "{err}");
    }

    // `Esc` on the first step cancels the flow. Steps answered by presets asked
    // nothing, so going back past them cancels too.
    let err = Flow::new(())
        .step("first", |_| Err(Error::Back))
        .run()
        .unwrap_err();
    assert!(matches!(err, Error::Cancelled));
    let err = Flow::new(())
        .step("preset", |_| name("Ada").interact_on(&term).map(drop))
        .step("second", |_| Err(Error::Back))
        .run()
        .unwrap_err();
    assert!(matches!(err, Error::Cancelled));
    assert_eq!(out.take(), "Name: Ada\n");

    // Other errors end the flow with them; `^C` is one of them.
    let err = Flow::new(())
        .step("a", |_| Err(Error::Cancelled))
        .step("b", |_| unreachable!())
        .run()
        .unwrap_err();
    assert!(matches!(err, Error::Cancelled));

    // One flow at a time: a flow inside a step is refused.
    let err = Flow::new(())
        .step("outer", |_| Flow::new(()).step("inner", |_| Ok(())).run())
        .run()
        .unwrap_err();
    assert_eq!(std::io::Error::from(err).kind(), ErrorKind::InvalidInput);
    // The registration is cleared on every exit, so the next flow runs.
    Flow::new(()).step("a", |_| Ok(())).run().unwrap();
    // Also after a panic in a step.
    let panicked = std::panic::catch_unwind(|| {
        let _ = Flow::new(()).step("a", |_| panic!("boom")).run();
    });
    assert!(panicked.is_err());
    Flow::new(()).step("a", |_| Ok(())).run().unwrap();

    // `Collapse` without a title acts like `Erase`, and outside a terminal nothing was
    // recorded, so nothing is erased.
    Flow::new(())
        .done(Done::Collapse)
        .step("a", |_| name("Ada").interact_on(&term).map(drop))
        .run()
        .unwrap();
    assert_eq!(out.take(), "Name: Ada\n");

    // `flow::printed` does nothing outside a flow.
    flow::printed("not recorded\n");
}
