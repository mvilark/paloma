//! What the prompt hooks of a [`Theme`](crate::theme::Theme) receive: one view per
//! layout, each saying which prompt it is drawn for.

/// Which prompt a view is drawn for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PromptKind {
    /// [`Confirm`](crate::Confirm).
    Confirm,
    /// [`Input`](crate::Input).
    Input,
    /// `Password` (feature `password`).
    Password,
    /// [`Select`](crate::Select).
    Select,
    /// [`MultiSelect`](crate::MultiSelect).
    MultiSelect,
    /// [`Sort`](crate::Sort).
    Sort,
    /// `FuzzySelect` (feature `fuzzy-select`).
    FuzzySelect,
}

/// The question line of a list prompt (select, multi select, sort); see
/// [`Theme::format_prompt`](crate::theme::Theme::format_prompt).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PromptView<'a> {
    /// Which prompt.
    pub kind: PromptKind,
    /// The question.
    pub prompt: &'a str,
    /// The current page and the number of pages, when the items do not fit.
    pub paging: Option<(usize, usize)>,
    /// The message row shows a validation error.
    pub invalid: bool,
}

impl<'a> PromptView<'a> {
    /// A view of the question line.
    pub fn new(
        kind: PromptKind,
        prompt: &'a str,
        paging: Option<(usize, usize)>,
        invalid: bool,
    ) -> Self {
        PromptView {
            kind,
            prompt,
            paging,
            invalid,
        }
    }
}

/// The line of an input or password prompt, before the typed text; see
/// [`Theme::format_input_prompt`](crate::theme::Theme::format_input_prompt).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct InputPromptView<'a> {
    /// [`PromptKind::Input`] or [`PromptKind::Password`].
    pub kind: PromptKind,
    /// The question.
    pub prompt: &'a str,
    /// The value `Enter` gives on an empty input, when it is shown.
    pub default: Option<&'a str>,
    /// The message row shows a validation error.
    pub invalid: bool,
}

impl<'a> InputPromptView<'a> {
    /// A view of the input line.
    pub fn new(kind: PromptKind, prompt: &'a str, default: Option<&'a str>, invalid: bool) -> Self {
        InputPromptView {
            kind,
            prompt,
            default,
            invalid,
        }
    }
}

/// The line of a confirm prompt; see
/// [`Theme::format_confirm_prompt`](crate::theme::Theme::format_confirm_prompt).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ConfirmPromptView<'a> {
    /// The question.
    pub prompt: &'a str,
    /// The answer `Enter` gives, when it is shown.
    pub default: Option<bool>,
    /// The prompt shows buttons on the line under it
    /// ([`Confirm::choices_as_buttons`](crate::Confirm::choices_as_buttons)), so this
    /// line holds only the question.
    pub buttons: bool,
    /// The message row shows a validation error.
    pub invalid: bool,
}

impl<'a> ConfirmPromptView<'a> {
    /// A view of the confirm line.
    pub fn new(prompt: &'a str, default: Option<bool>, buttons: bool, invalid: bool) -> Self {
        ConfirmPromptView {
            prompt,
            default,
            buttons,
            invalid,
        }
    }
}

/// The row of `Yes` / `No` buttons of a confirm prompt; see
/// [`Theme::format_confirm_buttons`](crate::theme::Theme::format_confirm_buttons).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ConfirmButtonsView {
    /// `Yes` has the focus (otherwise `No` has).
    pub yes_focused: bool,
}

impl ConfirmButtonsView {
    /// A view of the button row.
    pub fn new(yes_focused: bool) -> Self {
        ConfirmButtonsView { yes_focused }
    }
}

/// The search line of a fuzzy select; see
/// [`Theme::format_fuzzy_select_prompt`](crate::theme::Theme::format_fuzzy_select_prompt).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FuzzySelectPromptView<'a> {
    /// The question.
    pub prompt: &'a str,
    /// What the user typed.
    pub search: &'a str,
    /// The cursor, as a byte offset into `search`.
    pub cursor: usize,
    /// The placeholder to show in place of the search: `Some` only while the search
    /// is empty and the prompt has one
    /// (`FuzzySelect::placeholder`, feature `fuzzy-select`).
    pub placeholder: Option<&'a str>,
    /// The message row shows a validation error.
    pub invalid: bool,
}

impl<'a> FuzzySelectPromptView<'a> {
    /// A view of the search line.
    pub fn new(
        prompt: &'a str,
        search: &'a str,
        cursor: usize,
        placeholder: Option<&'a str>,
        invalid: bool,
    ) -> Self {
        FuzzySelectPromptView {
            prompt,
            search,
            cursor,
            placeholder,
            invalid,
        }
    }
}

/// One item of a list prompt; see
/// [`Theme::format_item`](crate::theme::Theme::format_item).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ItemView<'a> {
    /// Which prompt.
    pub kind: PromptKind,
    /// The item.
    pub text: &'a str,
    /// The item has the focus.
    pub active: bool,
    /// Whether the item is checked (multi select) or picked up to be moved (sort);
    /// `None` for prompts without checks.
    pub checked: Option<bool>,
    /// The characters (by index) that match the search, when a fuzzy select
    /// highlights matches.
    pub matches: Option<&'a [usize]>,
    /// Extra text to draw after the item ([`Item::description`](crate::Item::description)).
    pub description: Option<&'a str>,
    /// The item is disabled ([`Item::disabled`](crate::Item::disabled)): it is shown
    /// but cannot be focused or chosen.
    pub disabled: bool,
}

impl<'a> ItemView<'a> {
    /// A view of one item.
    pub fn new(
        kind: PromptKind,
        text: &'a str,
        active: bool,
        checked: Option<bool>,
        matches: Option<&'a [usize]>,
        description: Option<&'a str>,
        disabled: bool,
    ) -> Self {
        ItemView {
            kind,
            text,
            active,
            checked,
            matches,
            description,
            disabled,
        }
    }
}

/// The line of keys under a prompt; see
/// [`Theme::format_legend`](crate::theme::Theme::format_legend).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct LegendView<'a> {
    /// Which prompt.
    pub kind: PromptKind,
    /// The keys and what they do, as `(key, action)`: the prompt's `legend` token, plus
    /// the `legend_back` token while `Esc` goes back.
    pub keys: &'a [(&'a str, &'a str)],
}

impl<'a> LegendView<'a> {
    /// A view of the legend line.
    pub fn new(kind: PromptKind, keys: &'a [(&'a str, &'a str)]) -> Self {
        LegendView { kind, keys }
    }
}

/// Where the message row under a prompt got its message from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageSource {
    /// The prompt's `description(..)`, shown while nothing newer applies.
    Description,
    /// What the prompt's `on_change(..)` closure returned for the current value.
    Change,
    /// A failed validation (or a parse error), shown until the value changes.
    Validation,
}

/// One line of the message row under a prompt: the message or one of its nested
/// messages; see
/// [`Theme::format_prompt_message`](crate::theme::Theme::format_prompt_message).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PromptMessageView<'a> {
    /// Which prompt.
    pub kind: PromptKind,
    /// The message's kind.
    pub status: crate::Status,
    /// The message's text.
    pub text: &'a str,
    /// 0 for the message, 1 and more for nested messages.
    pub depth: usize,
    /// Where the message came from.
    pub source: MessageSource,
}

impl<'a> PromptMessageView<'a> {
    /// A view of one line of the message row.
    pub fn new(
        kind: PromptKind,
        status: crate::Status,
        text: &'a str,
        depth: usize,
        source: MessageSource,
    ) -> Self {
        PromptMessageView {
            kind,
            status,
            text,
            depth,
            source,
        }
    }
}

/// What a prompt was answered with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Answer<'a> {
    /// Text: an input, or the chosen item of a select.
    Text(&'a str),
    /// A confirm's answer.
    Bool(bool),
    /// The checked items of a multi select, or the order of a sort.
    List(&'a [&'a str]),
    /// A password, by its number of mask characters
    /// ([`Password::result_len`](crate::Password::result_len)).
    Hidden(usize),
    /// No answer: drawn as the `empty_value` token, nothing by default.
    Skipped,
    /// The user cancelled the prompt with `Ctrl+C`: drawn as the `cancelled_value`
    /// token in `cancelled_style`. The prompt draws this line only when the token is
    /// set.
    Cancelled,
}

/// The line an answered prompt leaves; see
/// [`Theme::format_answered`](crate::theme::Theme::format_answered).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AnsweredView<'a> {
    /// Which prompt.
    pub kind: PromptKind,
    /// The question.
    pub prompt: &'a str,
    /// The answer.
    pub answer: Answer<'a>,
}

impl<'a> AnsweredView<'a> {
    /// A view of the answered line.
    pub fn new(kind: PromptKind, prompt: &'a str, answer: Answer<'a>) -> Self {
        AnsweredView {
            kind,
            prompt,
            answer,
        }
    }
}
