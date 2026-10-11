use std::{io, ops::Rem};

use console::{Key, Term};
use fuzzy_matcher::FuzzyMatcher;

use crate::{
    output::message::{IntoMessage, Message, Status},
    prompts::{
        base::{base_methods, PromptBase},
        row::{Row, Validators},
    },
    theme::{
        render::{row_lines, TermThemeRenderer},
        {Answer, ItemView, PromptKind},
    },
    Result,
};

/// Renders a select prompt with fuzzy search.
///
/// User can use fuzzy search to limit selectable items.
/// Interaction returns index of an item selected in the order they appear in `item` invocation or `items` slice.
///
/// ## Example
///
/// ```rust,no_run
/// use paloma::FuzzySelect;
///
/// fn main() {
///     let items = vec!["foo", "bar", "baz"];
///
///     let selection = FuzzySelect::new()
///         .prompt("What do you choose?")
///         .items(&items)
///         .interact()
///         .unwrap();
///
///     println!("You chose: {}", items[selection]);
/// }
/// ```
#[derive(Clone)]
pub struct FuzzySelect<'a> {
    default: Option<usize>,
    items: Vec<String>,
    prompt: String,
    clear: bool,
    highlight_matches: bool,
    enable_vim_mode: bool,
    max_length: Option<usize>,
    base: PromptBase<'a>,
    /// Search string that a fuzzy search with start with.
    /// Defaults to an empty string.
    prefill: String,
    placeholder: Option<String>,
    validators: Validators<'a, usize>,
    row: Row<'a, usize>,
}

impl Default for FuzzySelect<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> FuzzySelect<'a> {
    /// Sets the clear behavior of the menu.
    ///
    /// The default is to clear the menu.
    pub fn clear(mut self, val: bool) -> Self {
        self.clear = val;
        self
    }

    /// Sets a default for the menu
    pub fn default(mut self, val: usize) -> Self {
        self.default = Some(val);
        self
    }

    /// Add a single item to the fuzzy selector.
    pub fn item<T: ToString>(mut self, item: T) -> Self {
        self.items.push(item.to_string());
        self
    }

    /// Adds multiple items to the fuzzy selector.
    pub fn items<T, I>(mut self, items: I) -> Self
    where
        T: ToString,
        I: IntoIterator<Item = T>,
    {
        self.items
            .extend(items.into_iter().map(|item| item.to_string()));

        self
    }

    /// Sets the search text that a fuzzy search starts with.
    pub fn prefill<S: Into<String>>(mut self, prefill: S) -> Self {
        self.prefill = prefill.into();
        self
    }

    /// Prefaces the menu with a prompt.
    ///
    /// When a prompt is set the system also prints out a confirmation after
    /// the fuzzy selection.
    pub fn prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.prompt = prompt.into();
        self
    }

    base_methods!(fuzzy_select, |_| true);

    /// Shows `text` in place of the search while it is empty, in the theme's
    /// `placeholder_style`. It goes away at the first key and is never searched for
    /// (unlike [`prefill`](Self::prefill)).
    pub fn placeholder<S: Into<String>>(mut self, text: S) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    /// Called each time the highlighted item changes, with its index in the items
    /// (not in the filtered list), once an item is highlighted. `Some(message)` shows
    /// in the row under the items; `None` brings the
    /// [`description`](Self::description) back.
    ///
    /// A later call replaces an earlier closure.
    pub fn on_change(
        mut self,
        f: impl FnMut(&usize) -> Option<Message<'static>> + Send + 'a,
    ) -> Self {
        self.row.on_change(f);
        self
    }

    /// Registers a validator, run with the chosen item's index (in the items) when the
    /// user picks it. An error keeps the prompt open and shows in the row under the
    /// items until the highlighted item changes. Several validators run in order; the
    /// first error wins.
    ///
    /// The error can be any `Display` value (shown as an error message) or a
    /// [`Message`] of any kind (see [`IntoMessage`]).
    pub fn validate<E: IntoMessage>(
        mut self,
        mut f: impl FnMut(&usize) -> std::result::Result<(), E> + 'a,
    ) -> Self {
        self.validators
            .push(move |i: &usize| f(i).err().map(|e| e.into_message(Status::Error)));
        self
    }

    /// Indicates whether to highlight matched indices
    ///
    /// The default is to highlight the indices
    pub fn highlight_matches(mut self, val: bool) -> Self {
        self.highlight_matches = val;
        self
    }

    /// Indicated whether to allow the use of vim mode
    ///
    /// Vim mode can be entered by pressing Escape.
    /// This then allows the user to navigate using hjkl.
    ///
    /// The default is to disable vim mode.
    pub fn vim_mode(mut self, val: bool) -> Self {
        self.enable_vim_mode = val;
        self
    }

    /// Sets the maximum number of visible options.
    ///
    /// The default is the height of the terminal minus 2.
    pub fn max_length(mut self, rows: usize) -> Self {
        self.max_length = Some(rows);
        self
    }

    /// Enables user interaction and returns the result.
    ///
    /// The user can select the items using 'Enter' and the index of selected item will be returned.
    /// The dialog is rendered on stderr.
    /// Result contains `index` of selected item if user hit 'Enter'.
    /// This unlike [`interact_opt`](Self::interact_opt) does not allow to quit with 'Esc' or 'q'.
    #[inline]
    pub fn interact(self) -> Result<usize> {
        self.interact_on(&Term::stderr())
    }

    /// Enables user interaction and returns the result.
    ///
    /// The user can select the items using 'Enter' and the index of selected item will be returned.
    /// The dialog is rendered on stderr.
    /// Result contains `Some(index)` if user hit 'Enter' or `None` if user cancelled with 'Esc' or 'q'.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use paloma::FuzzySelect;
    ///
    /// fn main() {
    ///     let items = vec!["foo", "bar", "baz"];
    ///
    ///     let selection = FuzzySelect::new()
    ///         .items(&items)
    ///         .interact_opt()
    ///         .unwrap();
    ///
    ///     match selection {
    ///         Some(index) => println!("You chose: {}", items[index]),
    ///         None => println!("You did not choose anything.")
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn interact_opt(self) -> Result<Option<usize>> {
        self.interact_on_opt(&Term::stderr())
    }

    /// Like [`interact`](Self::interact) but allows a specific terminal to be set.
    #[inline]
    pub fn interact_on(self, term: &Term) -> Result<usize> {
        Ok(self
            ._interact_on(term, false)?
            .ok_or_else(|| io::Error::other("Quit not allowed in this case"))?)
    }

    /// Like [`interact_opt`](Self::interact_opt) but allows a specific terminal to be set.
    #[inline]
    pub fn interact_on_opt(self, term: &Term) -> Result<Option<usize>> {
        self._interact_on(term, true)
    }

    fn _interact_on(mut self, term: &Term, allow_quit: bool) -> Result<Option<usize>> {
        // Place cursor at the end of the search term
        let mut cursor = self.prefill.chars().count();
        let mut search_term = self.prefill.to_owned();

        let mut render = TermThemeRenderer::new(term, self.base.theme);
        let mut sel = self.default;

        // Fuzzy matcher
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

        // Subtract -2 because we need space to render the prompt.
        let visible_term_rows = (term.size().0 as usize).max(3) - 2;
        let visible_term_rows = self
            .max_length
            .unwrap_or(visible_term_rows)
            .min(visible_term_rows);
        // Variable used to determine if we need to scroll through the list.
        let mut starting_row = 0;

        term.hide_cursor()?;

        let mut vim_mode = false;
        // The highlighted item (its index in the items) the row last heard of.
        let mut highlighted = None;
        let mut opened = false;

        loop {
            let mut byte_indices = search_term
                .char_indices()
                .map(|(index, _)| index)
                .collect::<Vec<_>>();

            byte_indices.push(search_term.len());

            // Maps all items to a tuple of item and its match score.
            let mut filtered_list = self
                .items
                .iter()
                .map(|item| (item, matcher.fuzzy_match(item, &search_term)))
                .filter_map(|(item, score)| score.map(|s| (item, s)))
                .collect::<Vec<_>>();

            // Renders all matching items, from best match to worst.
            filtered_list.sort_unstable_by(|(_, s1), (_, s2)| s2.cmp(s1));

            let now = sel
                .and_then(|sel| filtered_list.get(sel))
                .and_then(|(item, _)| self.index_of(item));
            if let (true, true, Some(now)) = (opened, now != highlighted, now) {
                self.row.changed(&now);
            }
            highlighted = now;
            opened = true;

            // The row takes rows from the list, so the whole prompt fits.
            let row = self.row.current(self.base.description.as_ref());
            let visible_rows = visible_term_rows.saturating_sub(row_lines(row)).max(1);
            if let Some(sel) = sel {
                if sel >= starting_row + visible_rows {
                    starting_row = sel + 1 - visible_rows;
                }
            }

            let shown: Vec<_> = filtered_list
                .iter()
                .enumerate()
                .skip(starting_row)
                .take(visible_rows)
                .map(|(idx, (item, _))| {
                    let matches = self
                        .highlight_matches
                        .then(|| matcher.fuzzy_indices(item, &search_term))
                        .flatten()
                        .map(|(_, indices)| indices);
                    (idx, *item, matches)
                })
                .collect();
            render.fuzzy_frame(
                self.prompt.as_str(),
                &search_term,
                byte_indices[cursor],
                self.placeholder.as_deref(),
                self.row.is_invalid(),
                shown.iter().map(|(idx, item, matches)| {
                    ItemView::new(
                        PromptKind::FuzzySelect,
                        item,
                        Some(*idx) == sel,
                        None,
                        matches.as_deref(),
                    )
                }),
                row,
            )?;

            match (term.read_key()?, sel, vim_mode) {
                (Key::Escape, _, false) if self.enable_vim_mode => {
                    vim_mode = true;
                }
                (Key::Escape, _, false) | (Key::Char('q'), _, true) if allow_quit => {
                    if self.clear {
                        render.erase()?;
                    } else {
                        render.commit()?;
                    }
                    term.flush()?;
                    term.show_cursor()?;
                    return Ok(None);
                }
                (Key::Char('i' | 'a'), _, true) => {
                    vim_mode = false;
                }
                (Key::ArrowUp | Key::BackTab, _, _) | (Key::Char('k'), _, true)
                    if !filtered_list.is_empty() =>
                {
                    if sel == Some(0) {
                        starting_row = filtered_list.len().max(visible_rows) - visible_rows;
                    } else if sel == Some(starting_row) {
                        starting_row -= 1;
                    }
                    sel = match sel {
                        None => Some(filtered_list.len() - 1),
                        Some(sel) => Some(
                            ((sel as i64 - 1 + filtered_list.len() as i64)
                                % (filtered_list.len() as i64))
                                as usize,
                        ),
                    };
                }
                (Key::ArrowDown | Key::Tab, _, _) | (Key::Char('j'), _, true)
                    if !filtered_list.is_empty() =>
                {
                    sel = match sel {
                        None => Some(0),
                        Some(sel) => {
                            Some((sel as u64 + 1).rem(filtered_list.len() as u64) as usize)
                        }
                    };
                    if sel == Some(visible_rows + starting_row) {
                        starting_row += 1;
                    } else if sel == Some(0) {
                        starting_row = 0;
                    }
                }
                (Key::ArrowLeft, _, _) | (Key::Char('h'), _, true) if cursor > 0 => {
                    cursor -= 1;
                }
                (Key::ArrowRight, _, _) | (Key::Char('l'), _, true)
                    if cursor < byte_indices.len() - 1 =>
                {
                    cursor += 1;
                }
                (Key::Enter, Some(sel), _) if !filtered_list.is_empty() => {
                    let sel_string = filtered_list[sel].0;
                    let sel_string_pos_in_items = self.index_of(sel_string);

                    if let Some(err) =
                        sel_string_pos_in_items.and_then(|i| self.validators.check(&i))
                    {
                        self.row.invalid(err);
                        continue;
                    }

                    if self.clear {
                        render.erase()?;
                    } else {
                        render.commit()?;
                    }

                    if self.report_enabled() {
                        render.answered(
                            PromptKind::FuzzySelect,
                            self.prompt.as_str(),
                            Answer::Text(sel_string),
                        )?;
                    }

                    term.show_cursor()?;
                    return Ok(sel_string_pos_in_items);
                }
                (Key::Backspace, _, _) if cursor > 0 => {
                    cursor -= 1;
                    search_term.remove(byte_indices[cursor]);
                }
                (Key::Del, _, _) if cursor < byte_indices.len() - 1 => {
                    search_term.remove(byte_indices[cursor]);
                }
                (Key::Char(chr), _, _) if !chr.is_ascii_control() => {
                    search_term.insert(byte_indices[cursor], chr);
                    cursor += 1;
                    sel = Some(0);
                    starting_row = 0;
                }

                _ => {}
            }
        }
    }

    /// The index in the items of `item` (the first item equal to it).
    fn index_of(&self, item: &str) -> Option<usize> {
        self.items.iter().position(|i| i == item)
    }
}

impl<'a> FuzzySelect<'a> {
    /// Creates a fuzzy select prompt drawn with the app theme (set with [`set_theme`](crate::set_theme), `SimpleTheme` by default).
    ///
    /// [`theme`](Self::theme) gives this prompt a theme of its own.
    pub fn new() -> Self {
        Self {
            default: None,
            items: vec![],
            prompt: "".into(),
            clear: true,
            highlight_matches: true,
            enable_vim_mode: false,
            max_length: None,
            base: PromptBase::new(),
            prefill: "".into(),
            placeholder: None,
            validators: Validators::default(),
            row: Row::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone() {
        let fuzzy_select = FuzzySelect::new().prompt("Do you want to continue?");

        let _ = fuzzy_select.clone();
    }

    #[test]
    fn test_iterator() {
        let items = ["First", "Second", "Third"];
        let iterator = items.iter().skip(1);

        assert_eq!(FuzzySelect::new().items(iterator).items, &items[1..]);
    }
}
