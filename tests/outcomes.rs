//! Presets and unattended runs of every prompt, on a terminal that is not a terminal.
//! `set_options` is global, so everything is one test function.
#![cfg(unix)]
mod common;

#[cfg(feature = "fuzzy-select")]
use paloma::FuzzySelect;
#[cfg(feature = "password")]
use paloma::Password;
use paloma::{
    theme::SimpleTheme, Confirm, Done, Error, Input, Item, MultiSelect, Options, Select, Sort,
};

use common::capture::term;

#[test]
fn unattended_prompts_answer_with_their_defaults() {
    // Cancelling returns `Error::Cancelled` here instead of exiting the test.
    paloma::set_options(paloma::Options::new().general(|g| g.cancel_exits(false)));
    paloma::set_options(
        Options::new()
            .general(|g| g.cancel_exits(false))
            .general(|g| g.unattended(true)),
    );
    let theme = SimpleTheme;
    let (term, out) = term();

    // Confirm and Input: the default, drawn as an answered prompt.
    let yes = Confirm::new()
        .theme(&theme)
        .prompt("Go?")
        .default(true)
        .interact_on(&term)
        .unwrap();
    assert!(yes);
    assert_eq!(out.take(), "Go? yes\n");
    let name: String = Input::new()
        .theme(&theme)
        .prompt("Name")
        .default("Ada".to_string())
        .interact_on(&term)
        .unwrap();
    assert_eq!(name, "Ada");
    assert_eq!(out.take(), "Name: Ada\n");

    // Select and FuzzySelect: the default; a disabled default moves to the next item.
    let i = Select::new()
        .theme(&theme)
        .prompt("Pick")
        .items(["a", "b", "c"])
        .default(1)
        .interact_on(&term)
        .unwrap();
    assert_eq!(i, 1);
    assert_eq!(out.take(), "Pick: b\n");
    let i = Select::new()
        .theme(&theme)
        .prompt("Pick")
        .item("a")
        .item(Item::new("b").disabled(true))
        .item("c")
        .default(1)
        .interact_on(&term)
        .unwrap();
    assert_eq!(i, 2);
    assert_eq!(out.take(), "Pick: c\n");
    #[cfg(feature = "fuzzy-select")]
    {
        let i = FuzzySelect::new()
            .theme(&theme)
            .prompt("Search")
            .items(["apple", "banana"])
            .default(1)
            .interact_on(&term)
            .unwrap();
        assert_eq!(i, 1);
        assert_eq!(out.take(), "Search: banana\n");
        let err = FuzzySelect::new().items(["apple"]).interact_on(&term);
        assert!(matches!(err, Err(Error::NoDefault)));
    }

    // MultiSelect: what is checked by default, possibly nothing; Sort: the order given.
    let picked = MultiSelect::new()
        .theme(&theme)
        .prompt("Features")
        .items_checked([("a", true), ("b", false), ("c", true)])
        .interact_on(&term)
        .unwrap();
    assert_eq!(picked, [0, 2]);
    assert_eq!(out.take(), "Features: a, c\n");
    let picked = MultiSelect::new()
        .theme(&theme)
        .prompt("Features")
        .items(["a", "b"])
        .interact_on(&term)
        .unwrap();
    assert!(picked.is_empty());
    assert_eq!(out.take(), "Features: \n");
    let order = Sort::new()
        .theme(&theme)
        .prompt("Rank")
        .items(["x", "y", "z"])
        .interact_on(&term)
        .unwrap();
    assert_eq!(order, [0, 1, 2]);
    assert_eq!(out.take(), "Rank: x, y, z\n");

    // Without a default there is nothing to answer with.
    let err = Confirm::new().prompt("Go?").interact_on(&term);
    assert!(matches!(err, Err(Error::NoDefault)));
    let err = Input::<String>::new().prompt("Name").interact_on(&term);
    assert!(matches!(err, Err(Error::NoDefault)));
    let err = Select::new().items(["a"]).interact_on(&term);
    assert!(matches!(err, Err(Error::NoDefault)));
    let err = Select::new()
        .item(Item::new("a").disabled(true))
        .default(0)
        .interact_on(&term);
    assert!(matches!(err, Err(Error::NoDefault)));
    #[cfg(feature = "password")]
    {
        let err = Password::new().prompt("Password").interact_on(&term);
        assert!(matches!(err, Err(Error::NoDefault)));
        assert_eq!(out.take(), "");
    }

    // A preset wins over unattended mode, and `done` applies to both.
    let yes = Confirm::new()
        .theme(&theme)
        .prompt("Go?")
        .default(true)
        .preset(Some(false))
        .interact_on(&term)
        .unwrap();
    assert!(!yes);
    assert_eq!(out.take(), "Go? no\n");
    let yes = Confirm::new()
        .theme(&theme)
        .prompt("Go?")
        .default(true)
        .done(Done::Erase)
        .interact_on(&term)
        .unwrap();
    assert!(yes);
    assert_eq!(out.take(), "");

    // The app option `prompts().done` reaches them too.
    paloma::set_options(
        Options::new()
            .general(|g| g.unattended(true))
            .prompts(|p| p.done(Done::Erase)),
    );
    let i = Select::new()
        .theme(&theme)
        .prompt("Pick")
        .items(["a", "b"])
        .default(0)
        .interact_on(&term)
        .unwrap();
    assert_eq!(i, 0);
    assert_eq!(out.take(), "");

    // Without unattended mode a prompt on a non-terminal is still an error.
    paloma::set_options(Options::new().general(|g| g.cancel_exits(false)));
    let err = Select::new().items(["a"]).default(0).interact_on(&term);
    assert!(matches!(err, Err(Error::IO(_))));
    let err = Confirm::new()
        .prompt("Go?")
        .default(true)
        .interact_on(&term);
    assert!(matches!(err, Err(Error::IO(_))));
}
