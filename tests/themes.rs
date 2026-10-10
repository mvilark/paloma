//! Golden output of every `Theme` hook for the built-in themes.
//!
//! Guards the built-in looks while the theme internals change (PAL-004): any difference
//! here is a visible change for every app that uses `SimpleTheme` or `ColorfulTheme`.
mod common;

use std::fmt::{self, Write};

use paloma::theme::{ColorfulTheme, SimpleTheme, Theme};

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

    #[cfg(feature = "password")]
    {
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
    }

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

    #[cfg(feature = "fuzzy-select")]
    {
        use fuzzy_matcher::skim::SkimMatcherV2;

        let matcher = SkimMatcherV2::default();
        for (active, highlight) in [(true, true), (true, false), (false, true), (false, false)] {
            s.line(
                &format!("fuzzy_select_prompt_item(banana, {active}, {highlight}, an)"),
                |f| {
                    theme.format_fuzzy_select_prompt_item(
                        f, "banana", active, highlight, &matcher, "an",
                    )
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
    }

    s.0
}

// One test function: colour support is global to the test binary.
#[test]
fn built_in_themes_render_unchanged() {
    console::set_colors_enabled(true);
    console::set_colors_enabled_stderr(true);
    common::assert_snapshot("theme-simple", &sheet(&SimpleTheme));
    common::assert_snapshot("theme-colorful", &sheet(&ColorfulTheme::default()));
}
