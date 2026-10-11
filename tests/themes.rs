//! Golden output of every `Theme` hook for the built-in themes, one snapshot per
//! theme and per optional feature, so the test passes whatever features are on.
//!
//! Guards the built-in looks while the theme internals change (PAL-004): any difference
//! here is a visible change for every app that uses `SimpleTheme` or `ColorfulTheme`.
mod common;

use std::fmt::{self, Write};

use paloma::{
    terminal::Stream,
    theme::{ColorfulTheme, Context, MessageView, SimpleTheme, Status, Theme},
    theme::{KeyValueView, PanelView, SectionView, TableView},
    PanelRow,
};

/// One line per hook call: `name(args) => output`.
struct Sheet(String);

impl Sheet {
    fn line(&mut self, call: &str, f: impl FnOnce(&mut dyn fmt::Write) -> fmt::Result) {
        let mut out = String::new();
        f(&mut out).unwrap();
        writeln!(self.0, "{call} => {out:?}").unwrap();
    }
}

fn sheet(theme: &dyn Theme) -> String {
    let mut s = Sheet(String::new());

    s.line("prompt(Name)", |f| theme.format_prompt(f, "Name"));
    s.line("prompt()", |f| theme.format_prompt(f, ""));
    s.line("error(Bad)", |f| theme.format_error(f, "Bad"));

    for default in [None, Some(true), Some(false)] {
        s.line(&format!("confirm_prompt(Go?, {default:?})"), |f| {
            theme.format_confirm_prompt(f, "Go?", default)
        });
    }
    s.line("confirm_prompt(, None)", |f| {
        theme.format_confirm_prompt(f, "", None)
    });
    s.line("confirm_prompt_no_hint(Go?)", |f| {
        theme.format_confirm_prompt_no_hint(f, "Go?")
    });
    for (yes, focused) in [(true, true), (true, false), (false, true), (false, false)] {
        s.line(&format!("confirm_button({yes}, {focused})"), |f| {
            theme.format_confirm_button(f, yes, focused)
        });
    }
    for yes_focused in [true, false] {
        s.line(&format!("confirm_buttons({yes_focused})"), |f| {
            theme.format_confirm_buttons(f, yes_focused)
        });
    }
    for sel in [Some(true), Some(false), None] {
        s.line(&format!("confirm_prompt_selection(Go?, {sel:?})"), |f| {
            theme.format_confirm_prompt_selection(f, "Go?", sel)
        });
        s.line(&format!("confirm_prompt_selection(, {sel:?})"), |f| {
            theme.format_confirm_prompt_selection(f, "", sel)
        });
    }

    for (prompt, default) in [("Name", None), ("Name", Some("Ada")), ("", Some("Ada"))] {
        s.line(&format!("input_prompt({prompt}, {default:?})"), |f| {
            theme.format_input_prompt(f, prompt, default)
        });
    }
    s.line("input_prompt_selection(Name, Ada)", |f| {
        theme.format_input_prompt_selection(f, "Name", "Ada")
    });

    s.line("select_prompt(Pick)", |f| {
        theme.format_select_prompt(f, "Pick")
    });
    s.line("select_prompt_selection(Pick, b)", |f| {
        theme.format_select_prompt_selection(f, "Pick", "b")
    });
    for active in [true, false] {
        s.line(&format!("select_prompt_item(b, {active})"), |f| {
            theme.format_select_prompt_item(f, "b", active)
        });
    }

    s.line("multi_select_prompt(Pick)", |f| {
        theme.format_multi_select_prompt(f, "Pick")
    });
    for sels in [&[][..], &["a", "b"][..]] {
        s.line(
            &format!("multi_select_prompt_selection(Pick, {sels:?})"),
            |f| theme.format_multi_select_prompt_selection(f, "Pick", sels),
        );
    }
    for (checked, active) in [(true, true), (true, false), (false, true), (false, false)] {
        s.line(
            &format!("multi_select_prompt_item(b, {checked}, {active})"),
            |f| theme.format_multi_select_prompt_item(f, "b", checked, active),
        );
    }

    s.line("sort_prompt(Rank)", |f| theme.format_sort_prompt(f, "Rank"));
    s.line("sort_prompt_selection(Rank, [a, b])", |f| {
        theme.format_sort_prompt_selection(f, "Rank", &["a", "b"])
    });
    for (picked, active) in [(true, true), (true, false), (false, true), (false, false)] {
        s.line(&format!("sort_prompt_item(b, {picked}, {active})"), |f| {
            theme.format_sort_prompt_item(f, "b", picked, active)
        });
    }

    s.0
}

#[cfg(feature = "password")]
fn password_sheet(theme: &dyn Theme) -> String {
    let mut s = Sheet(String::new());
    s.line("password_prompt(Password)", |f| {
        theme.format_password_prompt(f, "Password")
    });
    s.line("password_prompt_selection(Password)", |f| {
        theme.format_password_prompt_selection(f, "Password")
    });
    s.line("password_mask_glyph()", |f| {
        write!(f, "{}", theme.password_mask_glyph())
    });
    for len in [8, 3] {
        s.line(
            &format!("password_prompt_selection_masked(Password, {len})"),
            |f| theme.format_password_prompt_selection_masked(f, "Password", len),
        );
    }
    s.0
}

#[cfg(feature = "fuzzy-select")]
fn fuzzy_sheet(theme: &dyn Theme) -> String {
    use fuzzy_matcher::skim::SkimMatcherV2;

    let mut s = Sheet(String::new());
    let matcher = SkimMatcherV2::default();
    for (active, highlight) in [(true, true), (true, false), (false, true), (false, false)] {
        s.line(
            &format!("fuzzy_select_prompt_item(banana, {active}, {highlight}, an)"),
            |f| {
                theme
                    .format_fuzzy_select_prompt_item(f, "banana", active, highlight, &matcher, "an")
            },
        );
    }
    for pos in [0, 2, 3] {
        s.line(&format!("fuzzy_select_prompt(Search, ban, {pos})"), |f| {
            theme.format_fuzzy_select_prompt(f, "Search", "ban", pos)
        });
    }
    s.line("fuzzy_select_prompt(, ban, 3)", |f| {
        theme.format_fuzzy_select_prompt(f, "", "ban", 3)
    });
    s.0
}

fn message_sheet(theme: &dyn Theme) -> String {
    let mut s = Sheet(String::new());
    let tokens = theme.tokens();
    let ctx = Context::new(&tokens, Stream::Stderr);
    for status in [
        Status::Description,
        Status::Hint,
        Status::Info,
        Status::Success,
        Status::Warning,
        Status::Error,
    ] {
        for depth in [0, 1] {
            s.line(&format!("message({status:?}, Text, {depth})"), |f| {
                theme.format_message(f, &ctx, &MessageView::new(status, "Text", depth))
            });
        }
    }
    s.0
}

fn block_sheet(theme: &dyn Theme) -> String {
    let mut s = Sheet(String::new());
    let tokens = theme.tokens();
    let ctx = Context::new(&tokens, Stream::Stderr);
    s.line("section(Setup)", |f| {
        theme.format_section(f, &ctx, &SectionView::new("Setup"))
    });
    let rows = [
        ("State".to_string(), "Connected".to_string()),
        ("Server".into(), "es-bcn-01".into()),
    ];
    s.line("key_value", |f| {
        theme.format_key_value(f, &ctx, &KeyValueView::new(&rows))
    });
    let header = ["Server".to_string(), "Load".to_string()];
    let cells = vec![vec!["us-nyc-01".to_string(), "42%".to_string()]];
    s.line("table", |f| {
        theme.format_table(f, &ctx, &TableView::new(&header, &cells))
    });
    let panel = [
        PanelRow::new("Signed in")
            .status(Status::Success)
            .trailing("1.8s"),
        PanelRow::new("Kill switch is off").status(Status::Warning),
        PanelRow::new("Plain row"),
    ];
    s.line("panel(Done)", |f| {
        theme.format_panel(f, &ctx, &PanelView::new("Done", &panel))
    });
    s.0
}

// One test function: colour support is global to the test binary.
#[test]
fn built_in_themes_render_unchanged() {
    console::set_colors_enabled(true);
    console::set_colors_enabled_stderr(true);
    let colorful = ColorfulTheme::default();
    common::assert_snapshot("theme-simple", &sheet(&SimpleTheme));
    common::assert_snapshot("theme-colorful", &sheet(&colorful));
    common::assert_snapshot("theme-simple-messages", &message_sheet(&SimpleTheme));
    common::assert_snapshot("theme-colorful-messages", &message_sheet(&colorful));
    common::assert_snapshot("theme-simple-blocks", &block_sheet(&SimpleTheme));
    common::assert_snapshot("theme-colorful-blocks", &block_sheet(&colorful));
    // A restyled theme keeps every hook of the theme it wraps.
    let restyled = ColorfulTheme::default().restyle(|t| t);
    assert_eq!(sheet(&restyled), sheet(&colorful));
    assert_eq!(message_sheet(&restyled), message_sheet(&colorful));
    assert_eq!(block_sheet(&restyled), block_sheet(&colorful));
    #[cfg(feature = "password")]
    {
        common::assert_snapshot("theme-simple-password", &password_sheet(&SimpleTheme));
        common::assert_snapshot("theme-colorful-password", &password_sheet(&colorful));
    }
    #[cfg(feature = "fuzzy-select")]
    {
        common::assert_snapshot("theme-simple-fuzzy", &fuzzy_sheet(&SimpleTheme));
        common::assert_snapshot("theme-colorful-fuzzy", &fuzzy_sheet(&colorful));
    }
}
