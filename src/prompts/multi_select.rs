use std::{io, iter::repeat};

use console::{Key, Term};

use crate::{
    options::Done,
    output::message::{IntoMessage, Message, Status},
    paging::Paging,
    prompts::{
        base::{base_methods, PromptBase},
        item::{check_index, next_enabled, step_enabled, IntoItem, Item},
        row::{Keys, Row, Validators},
    },
    theme::{
        render::{row_lines, TermThemeRenderer},
        {Answer, ItemView, PromptKind},
    },
    Result,
};

/// Renders a multi select prompt.
///
/// ## Example
///
/// ```rust,no_run
/// use paloma::MultiSelect;
///
/// fn main() {
///     let items = vec!["foo", "bar", "baz"];
///
///     let selection = MultiSelect::new()
///         .prompt("What do you choose?")
///         .items(&items)
///         .interact()
///         .unwrap();
///
///     println!("You chose:");
///
///     for i in selection {
///         println!("{}", items[i]);
///     }
/// }
/// ```
#[derive(Clone)]
pub struct MultiSelect<'a> {
    defaults: Vec<bool>,
    items: Vec<Item>,
    prompt: Option<String>,
    preset: Option<Vec<usize>>,
    max_length: Option<usize>,
    base: PromptBase<'a>,
    validators: Validators<'a, [usize]>,
    row: Row<'a, [usize]>,
}

impl Default for MultiSelect<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> MultiSelect<'a> {
    /// Skips the prompt when `val` is `Some`: it returns those indices (the checked
    /// items) without needing a terminal, and draws the answered line as the prompt
    /// would have left it ([`done`](Self::done); `Keep` draws like `Collapse`). The
    /// preset is not validated and `on_change` is not called. `None` asks as usual.
    ///
    /// With `general().unattended(true)` and no preset, the prompt answers with the
    /// items checked by default.
    pub fn preset(mut self, val: Option<Vec<usize>>) -> Self {
        self.preset = val;
        self
    }

    /// Sets a defaults for the menu.
    pub fn defaults(mut self, val: &[bool]) -> Self {
        self.defaults = val
            .to_vec()
            .iter()
            .copied()
            .chain(repeat(false))
            .take(self.items.len())
            .collect();
        self
    }

    /// Sets an optional max length for a page
    ///
    /// Max length is disabled by None
    pub fn max_length(mut self, val: usize) -> Self {
        // Paging subtracts two from the capacity, paging does this to
        // make an offset for the page indicator. So to make sure that
        // we can show the intended amount of items we need to add two
        // to our value.
        self.max_length = Some(val + 2);
        self
    }

    /// Add a single item to the selector: an [`Item`] (with a description, or
    /// disabled) or any `Display` value. A disabled item keeps its checked state.
    #[inline]
    pub fn item(self, item: impl IntoItem) -> Self {
        self.item_checked(item, false)
    }

    /// Add a single item to the selector with a default checked state.
    pub fn item_checked(mut self, item: impl IntoItem, checked: bool) -> Self {
        self.items.push(item.into_item());
        self.defaults.push(checked);
        self
    }

    /// Adds multiple items to the selector.
    pub fn items<T, I>(self, items: I) -> Self
    where
        T: IntoItem,
        I: IntoIterator<Item = T>,
    {
        self.items_checked(items.into_iter().map(|item| (item, false)))
    }

    /// Adds multiple items to the selector with checked state
    pub fn items_checked<T, I>(mut self, items: I) -> Self
    where
        T: IntoItem,
        I: IntoIterator<Item = (T, bool)>,
    {
        for (item, checked) in items.into_iter() {
            self.items.push(item.into_item());
            self.defaults.push(checked);
        }
        self
    }

    /// Prefaces the menu with a prompt.
    ///
    /// By default, when a prompt is set the answer is left as one line after the
    /// selection. Change that with [`done`](Self::done).
    pub fn prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    base_methods!(multi_select);

    /// Called each time the checked items change, with their indices (ascending).
    /// `Some(message)` shows in the row under the items; `None` brings the
    /// [`description`](Self::description) back.
    ///
    /// A later call replaces an earlier closure.
    pub fn on_change(
        mut self,
        f: impl FnMut(&[usize]) -> Option<Message<'static>> + Send + 'a,
    ) -> Self {
        self.row.on_change(f);
        self
    }

    /// Registers a validator, run with the indices of the checked items (ascending) when the user submits. An error keeps the prompt open and shows in
    /// the row under the items until they change. Several validators run in order;
    /// the first error wins.
    ///
    /// The error can be any `Display` value (shown as an error message) or a
    /// [`Message`] of any kind (see [`IntoMessage`]).
    pub fn validate<E: IntoMessage>(
        mut self,
        mut f: impl FnMut(&[usize]) -> std::result::Result<(), E> + 'a,
    ) -> Self {
        self.validators
            .push(move |v: &[usize]| f(v).err().map(|e| e.into_message(Status::Error)));
        self
    }

    /// Enables user interaction and returns the result.
    ///
    /// The user can select the items with the 'Space' bar and on 'Enter' the indices of selected items will be returned.
    /// The dialog is rendered on stderr.
    /// Result contains `Vec<index>` if user hit 'Enter'.
    /// This unlike [`interact_opt`](Self::interact_opt) does not allow to quit with 'Esc' or 'q',
    /// except inside a [`Flow`](crate::Flow), where `Esc` or `q` returns [`Error::Back`](crate::Error::Back) (the step goes back).
    /// `Ctrl+C` returns [`Error::Cancelled`](crate::Error::Cancelled).
    #[inline]
    pub fn interact(self) -> Result<Vec<usize>> {
        self.interact_on(&Term::stderr())
    }

    /// Enables user interaction and returns the result.
    ///
    /// The user can select the items with the 'Space' bar and on 'Enter' the indices of selected items will be returned.
    /// The dialog is rendered on stderr.
    /// Result contains `Some(Vec<index>)` if user hit 'Enter' or `None` if user went back with 'Esc' or 'q' (the prompt is erased).
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use paloma::MultiSelect;
    ///
    /// fn main() {
    ///     let items = vec!["foo", "bar", "baz"];
    ///
    ///     let ordered = MultiSelect::new()
    ///         .items(&items)
    ///         .interact_opt()
    ///         .unwrap();
    ///
    ///     match ordered {
    ///         Some(positions) => {
    ///             println!("You chose:");
    ///
    ///             for i in positions {
    ///                 println!("{}", items[i]);
    ///             }
    ///         },
    ///         None => println!("You did not choose anything.")
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn interact_opt(self) -> Result<Option<Vec<usize>>> {
        self.interact_on_opt(&Term::stderr())
    }

    /// Like [`interact`](Self::interact) but allows a specific terminal to be set.
    #[inline]
    pub fn interact_on(self, term: &Term) -> Result<Vec<usize>> {
        let back = crate::flow::esc_back();
        crate::flow::required(self._interact_on(term, back), back)
    }

    /// Like [`interact_opt`](Self::interact_opt) but allows a specific terminal to be set.
    #[inline]
    pub fn interact_on_opt(self, term: &Term) -> Result<Option<Vec<usize>>> {
        self._interact_on(term, true)
    }

    fn _interact_on(mut self, term: &Term, allow_back: bool) -> Result<Option<Vec<usize>>> {
        let options = crate::options::current();
        let done = match self.done_mode(&options) {
            Done::Collapse if self.prompt.is_none() => Done::Erase,
            done => done,
        };

        // A preset or an unattended run answers without a terminal.
        let given = match &self.preset {
            Some(picked) => {
                for &i in picked {
                    check_index(i, self.items.len())?;
                }
                Some(picked.clone())
            }
            // The items checked by default (possibly none).
            None if options.unattended() => Some(picked(&self.defaults)),
            None => None,
        };
        if let Some(given) = given {
            if done != Done::Erase {
                let mut render = TermThemeRenderer::new(term, self.base.theme);
                let prompt = self.prompt.as_deref().unwrap_or("");
                let texts: Vec<&str> = given.iter().map(|&i| self.items[i].text()).collect();
                let answer = Answer::List(&texts);
                render.finish_preset(done, PromptKind::MultiSelect, prompt, answer)?;
            }
            return Ok(Some(given));
        }

        if !term.is_term() {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "not a terminal").into());
        }

        if self.items.is_empty() {
            Err(io::Error::other(
                "Empty list of items given to `MultiSelect`",
            ))?;
        }

        let mut paging = Paging::new(term, self.items.len(), self.max_length);
        let mut render = TermThemeRenderer::new(term, self.base.theme);
        render.set_legend(self.legend_enabled(&options), allow_back);
        // A flow going back to this prompt's step puts its message in the row.
        if let Some(message) = crate::flow::take_pending() {
            self.row.invalid(message);
        }
        // The highlighted item: the first enabled one (`!0` when every item is disabled).
        let mut sel = next_enabled(&self.items, 0, true).unwrap_or(!0);

        let mut checked: Vec<bool> = self.defaults.clone();

        term.hide_cursor()?;

        let worker = self.row.worker();
        Keys::run(worker, |keys| loop {
            self.draw(&mut render, &mut paging, sel, &checked)?;
            let before = checked.clone();

            let Some(key) = keys.next(term, &mut self.row)? else {
                continue;
            };
            match key {
                Key::CtrlC => return render.cancel(),
                Key::ArrowDown | Key::Tab | Key::Char('j') => {
                    sel = step_enabled(&self.items, sel, true).unwrap_or(sel);
                }
                Key::ArrowUp | Key::BackTab | Key::Char('k') => {
                    sel = step_enabled(&self.items, sel, false).unwrap_or(sel);
                }
                Key::ArrowLeft | Key::Char('h') => {
                    if paging.active {
                        let page = paging.previous_page();
                        sel = next_enabled(&self.items, page, true).unwrap_or(page);
                    }
                }
                Key::ArrowRight | Key::Char('l') => {
                    if paging.active {
                        let page = paging.next_page();
                        sel = next_enabled(&self.items, page, true).unwrap_or(page);
                    }
                }
                Key::Char(' ') if sel != !0 => {
                    checked[sel] = !checked[sel];
                }
                Key::Char('a') => {
                    // Disabled items keep their state.
                    let enabled = |i: &usize| !self.items[*i].is_disabled();
                    let all = (0..checked.len()).filter(enabled).all(|i| checked[i]);
                    for i in (0..checked.len()).filter(enabled) {
                        checked[i] = !all;
                    }
                }
                Key::Escape | Key::Char('q') => {
                    if allow_back {
                        render.back()?;
                        return Ok(None);
                    }
                }
                Key::Enter => {
                    let validates = !self.validators.is_empty();
                    if keys.defer_submit(&mut self.row, validates) {
                        continue;
                    }
                    if let Some(err) = self.validators.check(&picked(&checked)) {
                        self.row.invalid(err);
                        continue;
                    }

                    let selections: Vec<&str> = picked(&checked)
                        .into_iter()
                        .map(|idx| self.items[idx].text())
                        .collect();
                    let prompt = self.prompt.as_deref().unwrap_or("");
                    render.finish(
                        done,
                        PromptKind::MultiSelect,
                        prompt,
                        Answer::List(&selections),
                    )?;

                    return Ok(Some(picked(&checked)));
                }
                _ => {}
            }

            if checked != before {
                self.row.changed(&picked(&checked));
            }

            paging.update(sel);
        })
    }
}

impl MultiSelect<'_> {
    /// Draws the question, the items of the current page and the row.
    fn draw(
        &self,
        render: &mut TermThemeRenderer,
        paging: &mut Paging,
        sel: usize,
        checked: &[bool],
    ) -> Result {
        let row = self.row.current(self.base.description.as_ref());
        paging.set_reserved(
            row_lines(row) + render.legend_lines(PromptKind::MultiSelect),
            sel,
        );
        let items = self
            .items
            .iter()
            .enumerate()
            .skip(paging.current_page * paging.capacity)
            .take(paging.capacity)
            .map(|(idx, item)| {
                ItemView::new(
                    PromptKind::MultiSelect,
                    item.text(),
                    sel == idx,
                    Some(checked[idx]),
                    None,
                    item.description_text(),
                    item.is_disabled(),
                )
            });
        render.prompt_frame(
            PromptKind::MultiSelect,
            self.prompt.as_deref(),
            paging.counter(),
            self.row.is_invalid(),
            items,
            row,
        )
    }
}

/// The indices of the checked items, ascending.
fn picked(checked: &[bool]) -> Vec<usize> {
    checked
        .iter()
        .enumerate()
        .filter_map(|(idx, &checked)| checked.then_some(idx))
        .collect()
}

impl<'a> MultiSelect<'a> {
    /// Creates a multi select prompt drawn with the app theme (set with [`set_theme`](crate::set_theme), `SimpleTheme` by default).
    ///
    /// [`theme`](Self::theme) gives this prompt a theme of its own.
    pub fn new() -> Self {
        Self {
            items: vec![],
            defaults: vec![],
            prompt: None,
            preset: None,
            max_length: None,
            base: PromptBase::new(),
            validators: Validators::default(),
            row: Row::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(select: &MultiSelect) -> Vec<String> {
        select.items.iter().map(|i| i.text().to_string()).collect()
    }

    #[test]
    fn test_clone() {
        let multi_select = MultiSelect::new().prompt("Select your favorite(s)");

        let _ = multi_select.clone();
    }

    #[test]
    fn test_iterator() {
        let items = ["First", "Second", "Third"];
        let iterator = items.iter().skip(1);

        assert_eq!(texts(&MultiSelect::new().items(iterator)), &items[1..]);
    }

    #[test]
    fn disabled_items_keep_their_checked_state() {
        let select = MultiSelect::new()
            .item_checked(Item::new("a").disabled(true), true)
            .item("b");
        assert_eq!(select.defaults, [true, false]);
        assert_eq!(picked(&select.defaults), [0]);
    }

    #[cfg(unix)]
    mod unattended {
        use super::*;
        use crate::{prompts::testing::capture, theme::SimpleTheme};

        #[test]
        fn a_preset_answers_without_a_terminal() {
            let (term, out) = capture();
            let picked = MultiSelect::new()
                .theme(&SimpleTheme)
                .prompt("Features")
                .items(["a", "b", "c"])
                .preset(Some(vec![0, 2]))
                .interact_on(&term)
                .unwrap();
            assert_eq!(picked, [0, 2]);
            assert_eq!(out.text(), "Features: a, c\n");
        }

        #[test]
        fn a_preset_draws_as_done_says() {
            for (done, expected) in [
                (Done::Collapse, "Features: b\n"),
                (Done::Keep, "Features: b\n"),
                (Done::Erase, ""),
            ] {
                let (term, out) = capture();
                MultiSelect::new()
                    .theme(&SimpleTheme)
                    .prompt("Features")
                    .items(["a", "b"])
                    .done(done)
                    .preset(Some(vec![1]))
                    .interact_on(&term)
                    .unwrap();
                assert_eq!(out.text(), expected, "{done:?}");
            }
        }

        #[test]
        fn a_preset_that_names_no_item_is_an_error() {
            let (term, _) = capture();
            let err = MultiSelect::new()
                .items(["a"])
                .preset(Some(vec![3]))
                .interact_on(&term);
            assert!(matches!(err, Err(crate::Error::IO(_))));
        }
    }
}
