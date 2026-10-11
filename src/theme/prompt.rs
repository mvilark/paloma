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
}

impl<'a> PromptView<'a> {
    /// A view of the question line.
    pub fn new(kind: PromptKind, prompt: &'a str, paging: Option<(usize, usize)>) -> Self {
        PromptView {
            kind,
            prompt,
            paging,
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
}

impl<'a> InputPromptView<'a> {
    /// A view of the input line.
    pub fn new(kind: PromptKind, prompt: &'a str, default: Option<&'a str>) -> Self {
        InputPromptView {
            kind,
            prompt,
            default,
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
}

impl<'a> ConfirmPromptView<'a> {
    /// A view of the confirm line.
    pub fn new(prompt: &'a str, default: Option<bool>, buttons: bool) -> Self {
        ConfirmPromptView {
            prompt,
            default,
            buttons,
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
}

impl<'a> FuzzySelectPromptView<'a> {
    /// A view of the search line.
    pub fn new(prompt: &'a str, search: &'a str, cursor: usize) -> Self {
        FuzzySelectPromptView {
            prompt,
            search,
            cursor,
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
}

impl<'a> ItemView<'a> {
    /// A view of one item.
    pub fn new(
        kind: PromptKind,
        text: &'a str,
        active: bool,
        checked: Option<bool>,
        matches: Option<&'a [usize]>,
    ) -> Self {
        ItemView {
            kind,
            text,
            active,
            checked,
            matches,
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
    /// No answer (a confirm left with `Esc`).
    Skipped,
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
