//! Golden output of every `Theme` hook for the built-in themes, one snapshot per
//! theme and per optional feature, so the test passes whatever features are on.
//!
//! Guards the built-in looks while the theme internals change (PAL-004): any difference
//! here is a visible change for every app that uses `SimpleTheme` or `ColorfulTheme`.
mod common;

use std::{
    fmt::{self, Write},
    time::Duration,
};

use paloma::{
    terminal::Stream,
    theme::{
        Answer, AnsweredView, ConfirmButtonsView, ConfirmPromptView, InputPromptView, ItemView,
        PromptKind, PromptView,
    },
    theme::{ColorfulTheme, Context, MessageView, SimpleTheme, Status, Theme},
    theme::{KeyValueView, PanelView, SectionView, TableView, TaskState, TaskStepView, TaskView},
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
    let tokens = theme.tokens();
    let ctx = &Context::new(&tokens, Stream::Stderr);
    let answered = |f: &mut dyn fmt::Write, kind, prompt, answer| {
        theme.format_answered(f, ctx, &AnsweredView::new(kind, prompt, answer))
    };
    let item = |f: &mut dyn fmt::Write, kind, active, checked| {
        theme.format_item(f, ctx, &ItemView::new(kind, "b", active, checked, None))
    };

    s.line("prompt(Name)", |f| {
        theme.format_prompt(f, ctx, &PromptView::new(PromptKind::Select, "Name", None))
    });
    s.line("prompt()", |f| {
        theme.format_prompt(f, ctx, &PromptView::new(PromptKind::Select, "", None))
    });
    s.line("error(Bad)", |f| {
        theme.format_message(f, ctx, &MessageView::new(Status::Error, "Bad", 0))
    });

    for default in [None, Some(true), Some(false)] {
        s.line(&format!("confirm_prompt(Go?, {default:?})"), |f| {
            theme.format_confirm_prompt(f, ctx, &ConfirmPromptView::new("Go?", default, false))
        });
    }
    s.line("confirm_prompt(, None)", |f| {
        theme.format_confirm_prompt(f, ctx, &ConfirmPromptView::new("", None, false))
    });
    s.line("confirm_prompt_no_hint(Go?)", |f| {
        theme.format_confirm_prompt(f, ctx, &ConfirmPromptView::new("Go?", None, true))
    });
    s.line("confirm_prompt_no_hint()", |f| {
        theme.format_confirm_prompt(f, ctx, &ConfirmPromptView::new("", None, true))
    });
    for yes_focused in [true, false] {
        s.line(&format!("confirm_buttons({yes_focused})"), |f| {
            theme.format_confirm_buttons(f, ctx, &ConfirmButtonsView::new(yes_focused))
        });
    }
    for sel in [Some(true), Some(false), None] {
        let answer = sel.map_or(Answer::Skipped, Answer::Bool);
        s.line(&format!("confirm_prompt_selection(Go?, {sel:?})"), |f| {
            answered(f, PromptKind::Confirm, "Go?", answer)
        });
        s.line(&format!("confirm_prompt_selection(, {sel:?})"), |f| {
            answered(f, PromptKind::Confirm, "", answer)
        });
    }

    for (prompt, default) in [("Name", None), ("Name", Some("Ada")), ("", Some("Ada"))] {
        s.line(&format!("input_prompt({prompt}, {default:?})"), |f| {
            let view = InputPromptView::new(PromptKind::Input, prompt, default);
            theme.format_input_prompt(f, ctx, &view)
        });
    }
    s.line("input_prompt_selection(Name, Ada)", |f| {
        answered(f, PromptKind::Input, "Name", Answer::Text("Ada"))
    });

    s.line("select_prompt(Pick)", |f| {
        theme.format_prompt(f, ctx, &PromptView::new(PromptKind::Select, "Pick", None))
    });
    s.line("select_prompt(Pick, page 1/2)", |f| {
        let view = PromptView::new(PromptKind::Select, "Pick", Some((1, 2)));
        theme.format_prompt(f, ctx, &view)
    });
    s.line("select_prompt_selection(Pick, b)", |f| {
        answered(f, PromptKind::Select, "Pick", Answer::Text("b"))
    });
    for active in [true, false] {
        s.line(&format!("select_prompt_item(b, {active})"), |f| {
            item(f, PromptKind::Select, active, None)
        });
    }

    s.line("multi_select_prompt(Pick)", |f| {
        let view = PromptView::new(PromptKind::MultiSelect, "Pick", None);
        theme.format_prompt(f, ctx, &view)
    });
    for sels in [&[][..], &["a", "b"][..]] {
        s.line(
            &format!("multi_select_prompt_selection(Pick, {sels:?})"),
            |f| answered(f, PromptKind::MultiSelect, "Pick", Answer::List(sels)),
        );
    }
    for (checked, active) in [(true, true), (true, false), (false, true), (false, false)] {
        s.line(
            &format!("multi_select_prompt_item(b, {checked}, {active})"),
            |f| item(f, PromptKind::MultiSelect, active, Some(checked)),
        );
    }

    s.line("sort_prompt(Rank)", |f| {
        theme.format_prompt(f, ctx, &PromptView::new(PromptKind::Sort, "Rank", None))
    });
    s.line("sort_prompt_selection(Rank, [a, b])", |f| {
        answered(f, PromptKind::Sort, "Rank", Answer::List(&["a", "b"]))
    });
    for (picked, active) in [(true, true), (true, false), (false, true), (false, false)] {
        s.line(&format!("sort_prompt_item(b, {picked}, {active})"), |f| {
            item(f, PromptKind::Sort, active, Some(picked))
        });
    }

    s.0
}

#[cfg(feature = "password")]
fn password_sheet(theme: &dyn Theme) -> String {
    let mut s = Sheet(String::new());
    let tokens = theme.tokens();
    let ctx = &Context::new(&tokens, Stream::Stderr);
    s.line("password_prompt(Password)", |f| {
        let view = InputPromptView::new(PromptKind::Password, "Password", None);
        theme.format_input_prompt(f, ctx, &view)
    });
    s.line("password_mask_glyph()", |f| {
        let look = tokens.get_prompts().resolve(PromptKind::Password, &tokens);
        write!(f, "{}", look.mask)
    });
    for len in [8, 3] {
        s.line(
            &format!("password_prompt_selection_masked(Password, {len})"),
            |f| {
                let view = AnsweredView::new(PromptKind::Password, "Password", Answer::Hidden(len));
                theme.format_answered(f, ctx, &view)
            },
        );
    }
    s.0
}

#[cfg(feature = "fuzzy-select")]
use paloma::theme::FuzzySelectPromptView;

#[cfg(feature = "fuzzy-select")]
fn fuzzy_sheet(theme: &dyn Theme) -> String {
    use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

    let mut s = Sheet(String::new());
    let tokens = theme.tokens();
    let ctx = &Context::new(&tokens, Stream::Stderr);
    let matcher = SkimMatcherV2::default();
    for (active, highlight) in [(true, true), (true, false), (false, true), (false, false)] {
        s.line(
            &format!("fuzzy_select_prompt_item(banana, {active}, {highlight}, an)"),
            |f| {
                let matches = highlight
                    .then(|| matcher.fuzzy_indices("banana", "an"))
                    .flatten()
                    .map(|(_, indices)| indices);
                let view = ItemView::new(
                    PromptKind::FuzzySelect,
                    "banana",
                    active,
                    None,
                    matches.as_deref(),
                );
                theme.format_item(f, ctx, &view)
            },
        );
    }
    for pos in [0, 2, 3] {
        s.line(&format!("fuzzy_select_prompt(Search, ban, {pos})"), |f| {
            let view = FuzzySelectPromptView::new("Search", "ban", pos);
            theme.format_fuzzy_select_prompt(f, ctx, &view)
        });
    }
    s.line("fuzzy_select_prompt(, ban, 3)", |f| {
        theme.format_fuzzy_select_prompt(f, ctx, &FuzzySelectPromptView::new("", "ban", 3))
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

fn task_sheet(theme: &dyn Theme) -> String {
    let mut s = Sheet(String::new());
    let tokens = theme.tokens();
    let ctx = Context::new(&tokens, Stream::Stderr);
    let elapsed = Duration::from_millis(1250);
    for frame in [0, 3] {
        s.line(&format!("task(Working, running, {frame})"), |f| {
            let view = TaskView::new("Working", TaskState::Running, elapsed, frame);
            theme.format_task(f, &ctx, &view)
        });
    }
    for status in [
        Status::Description,
        Status::Hint,
        Status::Info,
        Status::Success,
        Status::Warning,
        Status::Error,
    ] {
        s.line(&format!("task(Text, {status:?})"), |f| {
            let view = TaskView::new("Text", TaskState::Finished(status), elapsed, 0);
            theme.format_task(f, &ctx, &view)
        });
    }
    // What `restyle(|t| t.live(|l| l.task(|t| t.show_elapsed(false))))` changes: hooks
    // read their tokens from the context.
    let quiet = tokens
        .clone()
        .into_owned()
        .live(|l| l.task(|t| t.show_elapsed(false)));
    let quiet_ctx = Context::new(&quiet, Stream::Stderr);
    s.line("task(Working, running, 0, no elapsed)", |f| {
        let view = TaskView::new("Working", TaskState::Running, elapsed, 0);
        theme.format_task(f, &quiet_ctx, &view)
    });
    s.line("task_step(Generated device key)", |f| {
        theme.format_task_step(f, &ctx, &TaskStepView::new("Generated device key"))
    });
    s.0
}

// One test function: colour support is global to the test binary.
#[test]
fn built_in_themes_render_unchanged() {
    console::set_colors_enabled(true);
    console::set_colors_enabled_stderr(true);
    let colorful = ColorfulTheme;
    common::assert_snapshot("theme-simple", &sheet(&SimpleTheme));
    common::assert_snapshot("theme-colorful", &sheet(&colorful));
    common::assert_snapshot("theme-simple-messages", &message_sheet(&SimpleTheme));
    common::assert_snapshot("theme-colorful-messages", &message_sheet(&colorful));
    common::assert_snapshot("theme-simple-blocks", &block_sheet(&SimpleTheme));
    common::assert_snapshot("theme-colorful-blocks", &block_sheet(&colorful));
    common::assert_snapshot("theme-simple-tasks", &task_sheet(&SimpleTheme));
    common::assert_snapshot("theme-colorful-tasks", &task_sheet(&colorful));
    // A restyled theme keeps every hook of the theme it wraps.
    let restyled = ColorfulTheme.restyle(|t| t);
    assert_eq!(sheet(&restyled), sheet(&colorful));
    assert_eq!(message_sheet(&restyled), message_sheet(&colorful));
    assert_eq!(block_sheet(&restyled), block_sheet(&colorful));
    assert_eq!(task_sheet(&restyled), task_sheet(&colorful));
    // `restyle` reaches the live tokens: a task without the elapsed time.
    let quiet = ColorfulTheme.restyle(|t| t.live(|l| l.task(|s| s.show_elapsed(false))));
    let running_line = |theme: &dyn Theme| {
        let tokens = theme.tokens();
        let ctx = Context::new(&tokens, Stream::Stderr);
        let view = TaskView::new("Working", TaskState::Running, Duration::from_secs(1), 0);
        let mut out = String::new();
        theme.format_task(&mut out, &ctx, &view).unwrap();
        out
    };
    assert!(running_line(&colorful).contains("1.0s"));
    assert!(!running_line(&quiet).contains("1.0s"));
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
