use std::io;

use console::{Key, Term};
use fuzzy_matcher::FuzzyMatcher;

use crate::{
    options::Done,
    output::message::{IntoMessage, Message, Status},
    prompts::{
        base::{base_methods, PromptBase},
        item::{check_index, next_enabled, next_where, IntoItem, Item},
        row::{Keys, Row, Validators},
    },
    theme::{
        render::{row_lines, TermThemeRenderer},
        {Answer, ItemView, PromptKind},
    },
    Error, Result,
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
    items: Vec<Item>,
    prompt: String,
    preset: Option<usize>,
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
    /// Sets a default for the menu: the position in the list when it opens (an item
    /// index while the search is empty). A disabled item starts on the next enabled
    /// one. With `general().unattended(true)` the prompt answers with the item of this
    /// index.
    pub fn default(mut self, val: usize) -> Self {
        self.default = Some(val);
        self
    }

    /// Skips the prompt when `val` is `Some`: it returns that item's index without
    /// needing a terminal, and draws the answered line as the prompt would have left
    /// it ([`done`](Self::done); `Keep` draws like `Collapse`). The preset is not
    /// validated and `on_change` is not called. `None` asks as usual.
    pub fn preset(mut self, val: Option<usize>) -> Self {
        self.preset = val;
        self
    }

    /// Add a single item to the fuzzy selector: an [`Item`] (with a description, or
    /// disabled) or any `Display` value. Items are matched on their text.
    pub fn item(mut self, item: impl IntoItem) -> Self {
        self.items.push(item.into_item());
        self
    }

    /// Adds multiple items to the fuzzy selector (see [`item`](Self::item)).
    pub fn items<T, I>(mut self, items: I) -> Self
    where
        T: IntoItem,
        I: IntoIterator<Item = T>,
    {
        self.items
            .extend(items.into_iter().map(IntoItem::into_item));

        self
    }

    /// Sets the search text that a fuzzy search starts with.
    pub fn prefill<S: Into<String>>(mut self, prefill: S) -> Self {
        self.prefill = prefill.into();
        self
    }

    /// Prefaces the menu with a prompt.
    ///
    /// The answer is left as one line after the fuzzy selection; change that with
    /// [`done`](Self::done).
    pub fn prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.prompt = prompt.into();
        self
    }

    base_methods!(fuzzy_select);

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
    /// This unlike [`interact_opt`](Self::interact_opt) does not allow to quit with 'Esc' or 'q',
    /// except inside a [`Flow`](crate::Flow), where `Esc` returns [`Error::Back`] (the step goes back).
    /// `Ctrl+C` returns [`Error::Cancelled`].
    #[inline]
    pub fn interact(self) -> Result<usize> {
        self.interact_on(&Term::stderr())
    }

    /// Enables user interaction and returns the result.
    ///
    /// The user can select the items using 'Enter' and the index of selected item will be returned.
    /// The dialog is rendered on stderr.
    /// Result contains `Some(index)` if user hit 'Enter' or `None` if user went back with 'Esc' or 'q' (the prompt is erased; with
    /// [`vim_mode`](Self::vim_mode) `Esc` enters vim mode and `q` goes back).
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
        let back = crate::flow::esc_back();
        crate::flow::required(self._interact_on(term, back), back)
    }

    /// Like [`interact_opt`](Self::interact_opt) but allows a specific terminal to be set.
    #[inline]
    pub fn interact_on_opt(self, term: &Term) -> Result<Option<usize>> {
        self._interact_on(term, true)
    }

    fn _interact_on(mut self, term: &Term, allow_back: bool) -> Result<Option<usize>> {
        let options = crate::options::current();
        let done = self.done_mode(&options);

        // A preset or an unattended run answers without a terminal.
        let given = match self.preset {
            Some(sel) => Some(check_index(sel, self.items.len())?),
            None if options.unattended() => Some(self.unattended()?),
            None => None,
        };
        if let Some(sel) = given {
            if done != Done::Erase {
                let mut render = TermThemeRenderer::new(term, self.base.theme);
                let answer = Answer::Text(self.items[sel].text());
                render.finish_preset(done, PromptKind::FuzzySelect, &self.prompt, answer)?;
            }
            return Ok(Some(sel));
        }

        if !term.is_term() {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "not a terminal").into());
        }

        // Place cursor at the end of the search term
        let mut cursor = self.prefill.chars().count();
        let mut search_term = self.prefill.to_owned();

        let mut render = TermThemeRenderer::new(term, self.base.theme);
        render.set_legend(self.legend_enabled(&options), allow_back);
        // A flow going back to this prompt's step puts its message in the row.
        if let Some(message) = crate::flow::take_pending() {
            self.row.invalid(message);
        }
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

        let worker = self.row.worker();
        Keys::run(worker, |keys| loop {
            let mut byte_indices = search_term
                .char_indices()
                .map(|(index, _)| index)
                .collect::<Vec<_>>();

            byte_indices.push(search_term.len());

            // Maps all items to a tuple of item and its match score.
            let mut filtered_list = self
                .items
                .iter()
                .enumerate()
                .map(|(idx, item)| (idx, item, matcher.fuzzy_match(item.text(), &search_term)))
                .filter_map(|(idx, item, score)| score.map(|s| (idx, item, s)))
                .collect::<Vec<_>>();

            // Renders all matching items, from best match to worst.
            filtered_list.sort_unstable_by(|(_, _, s1), (_, _, s2)| s2.cmp(s1));

            // The highlight never rests on a disabled item.
            if let Some(at) = sel.filter(|_| !filtered_list.is_empty()) {
                let enabled = |i: usize| !filtered_list[i].1.is_disabled();
                let len = filtered_list.len();
                sel = next_where(len, at, true, enabled);
            }

            let now = sel
                .and_then(|sel| filtered_list.get(sel))
                .map(|(idx, ..)| *idx);
            if let (true, true, Some(now)) = (opened, now != highlighted, now) {
                self.row.changed(&now);
            }
            highlighted = now;
            opened = true;

            // The row and the legend take rows from the list, so the whole prompt fits.
            let row = self.row.current(self.base.description.as_ref());
            let reserved = row_lines(row) + render.legend_lines(PromptKind::FuzzySelect);
            let visible_rows = visible_term_rows.saturating_sub(reserved).max(1);
            if let Some(sel) = sel {
                if sel >= starting_row + visible_rows {
                    starting_row = sel + 1 - visible_rows;
                } else if sel < starting_row {
                    starting_row = sel;
                }
            }

            let shown: Vec<_> = filtered_list
                .iter()
                .enumerate()
                .skip(starting_row)
                .take(visible_rows)
                .map(|(pos, (_, item, _))| {
                    let matches = self
                        .highlight_matches
                        .then(|| matcher.fuzzy_indices(item.text(), &search_term))
                        .flatten()
                        .map(|(_, indices)| indices);
                    (pos, *item, matches)
                })
                .collect();
            render.fuzzy_frame(
                self.prompt.as_str(),
                &search_term,
                byte_indices[cursor],
                self.placeholder.as_deref(),
                self.row.is_invalid(),
                shown.iter().map(|(pos, item, matches)| {
                    ItemView::new(
                        PromptKind::FuzzySelect,
                        item.text(),
                        Some(*pos) == sel,
                        None,
                        matches.as_deref(),
                        item.description_text(),
                        item.is_disabled(),
                    )
                }),
                row,
            )?;

            let Some(key) = keys.next(term, &mut self.row)? else {
                continue;
            };
            match (key, sel, vim_mode) {
                (Key::CtrlC, ..) => {
                    return render.cancel(PromptKind::FuzzySelect, Some(&self.prompt))
                }
                (Key::Escape, _, false) if self.enable_vim_mode => {
                    vim_mode = true;
                }
                (Key::Escape, _, false) | (Key::Char('q'), _, true) if allow_back => {
                    render.back()?;
                    return Ok(None);
                }
                (Key::Char('i' | 'a'), _, true) => {
                    vim_mode = false;
                }
                (Key::ArrowUp | Key::BackTab, _, _) | (Key::Char('k'), _, true)
                    if !filtered_list.is_empty() =>
                {
                    let len = filtered_list.len();
                    let enabled = |i: usize| !filtered_list[i].1.is_disabled();
                    let from = sel.map_or(len - 1, |sel| (sel + len - 1) % len);
                    sel = next_where(len, from, false, enabled).or(sel);
                }
                (Key::ArrowDown | Key::Tab, _, _) | (Key::Char('j'), _, true)
                    if !filtered_list.is_empty() =>
                {
                    let len = filtered_list.len();
                    let enabled = |i: usize| !filtered_list[i].1.is_disabled();
                    let from = sel.map_or(0, |sel| (sel + 1) % len);
                    sel = next_where(len, from, true, enabled).or(sel);
                }
                (Key::ArrowLeft, _, _) | (Key::Char('h'), _, true) if cursor > 0 => {
                    cursor -= 1;
                }
                (Key::ArrowRight, _, _) | (Key::Char('l'), _, true)
                    if cursor < byte_indices.len() - 1 =>
                {
                    cursor += 1;
                }
                (Key::Enter, Some(sel), _) if sel < filtered_list.len() => {
                    let (index, item, _) = filtered_list[sel];

                    let validates = !self.validators.is_empty();
                    if keys.defer_submit(&mut self.row, validates) {
                        continue;
                    }
                    if let Some(err) = self.validators.check(&index) {
                        self.row.invalid(err);
                        continue;
                    }

                    render.finish(
                        done,
                        PromptKind::FuzzySelect,
                        &self.prompt,
                        Answer::Text(item.text()),
                    )?;

                    return Ok(Some(index));
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
        })
    }

    /// The index an unattended run answers with: the item at the default's index, or the
    /// next enabled item when that one is disabled.
    fn unattended(&self) -> Result<usize> {
        let default = self.default.ok_or(Error::NoDefault)?;
        let default = check_index(default, self.items.len())?;
        next_enabled(&self.items, default, true).ok_or(Error::NoDefault)
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
            preset: None,
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

        let texts: Vec<_> = FuzzySelect::new()
            .items(iterator)
            .items
            .iter()
            .map(|i| i.text().to_string())
            .collect();
        assert_eq!(texts, &items[1..]);
    }

    #[cfg(unix)]
    mod unattended {
        use super::*;
        use crate::{prompts::testing::capture, theme::SimpleTheme};

        #[test]
        fn a_preset_answers_without_a_terminal() {
            let (term, out) = capture();
            let picked = FuzzySelect::new()
                .theme(&SimpleTheme)
                .prompt("Search")
                .items(["apple", "banana"])
                .preset(Some(1))
                .interact_on(&term)
                .unwrap();
            assert_eq!(picked, 1);
            assert_eq!(out.text(), "Search: banana\n");
        }

        #[test]
        fn a_preset_draws_as_done_says() {
            for (done, expected) in [
                (Done::Collapse, "Search: apple\n"),
                (Done::Keep, "Search: apple\n"),
                (Done::Erase, ""),
            ] {
                let (term, out) = capture();
                FuzzySelect::new()
                    .theme(&SimpleTheme)
                    .prompt("Search")
                    .items(["apple", "banana"])
                    .done(done)
                    .preset(Some(0))
                    .interact_on(&term)
                    .unwrap();
                assert_eq!(out.text(), expected, "{done:?}");
            }
        }

        #[test]
        fn an_unattended_run_needs_a_default() {
            let select = FuzzySelect::new().items(["apple", "banana"]);
            assert!(matches!(select.unattended(), Err(Error::NoDefault)));
            let select = select.default(1);
            assert_eq!(select.unattended().unwrap(), 1);
        }

        #[test]
        fn a_prompt_on_a_non_terminal_is_an_error() {
            let (term, _) = capture();
            let err = FuzzySelect::new().items(["apple"]).interact_on(&term);
            assert!(matches!(err, Err(Error::IO(_))));
        }
    }
}
