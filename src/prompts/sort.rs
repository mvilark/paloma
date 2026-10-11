use std::io;

use console::{Key, Term};

use crate::{
    options::Done,
    output::message::{IntoMessage, Message, Status},
    paging::Paging,
    prompts::{
        base::{base_methods, PromptBase},
        item::check_index,
        row::{Keys, Row, Validators},
    },
    theme::{
        render::{row_lines, TermThemeRenderer},
        {Answer, ItemView, PromptKind},
    },
    Result,
};

/// Renders a sort prompt.
///
/// Returns list of indices in original items list sorted according to user input.
///
/// ## Example
///
/// ```rust,no_run
/// use paloma::Sort;
///
/// fn main() {
///     let items = vec!["foo", "bar", "baz"];
///
///     let ordered = Sort::new()
///         .prompt("Which order do you prefer?")
///         .items(&items)
///         .interact()
///         .unwrap();
///
///     println!("You prefer:");
///
///     for i in ordered {
///         println!("{}", items[i]);
///     }
/// }
/// ```
#[derive(Clone)]
pub struct Sort<'a> {
    items: Vec<String>,
    prompt: Option<String>,
    preset: Option<Vec<usize>>,
    max_length: Option<usize>,
    base: PromptBase<'a>,
    validators: Validators<'a, [usize]>,
    row: Row<'a, [usize]>,
}

impl Default for Sort<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Sort<'a> {
    /// Skips the prompt when `val` is `Some`: it returns that order (item indices,
    /// first to last) without needing a terminal, and draws the answered line as the
    /// prompt would have left it ([`done`](Self::done); `Keep` draws like `Collapse`).
    /// The preset is not validated and `on_change` is not called. `None` asks as usual.
    ///
    /// With `general().unattended(true)` and no preset, the prompt answers with the
    /// original order.
    pub fn preset(mut self, val: Option<Vec<usize>>) -> Self {
        self.preset = val;
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

    /// Add a single item to the selector.
    pub fn item<T: ToString>(mut self, item: T) -> Self {
        self.items.push(item.to_string());
        self
    }

    /// Adds multiple items to the selector.
    pub fn items<T, I>(mut self, items: I) -> Self
    where
        T: ToString,
        I: IntoIterator<Item = T>,
    {
        self.items
            .extend(items.into_iter().map(|item| item.to_string()));

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

    base_methods!(sort);

    /// Called each time the order changes, with the item indices from first to last.
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

    /// Registers a validator, run with the order (item indices, first to last) when the user submits. An error keeps the prompt open and shows in
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
    /// The user can order the items with the 'Space' bar and the arrows. On 'Enter' ordered list of the incides of items will be returned.
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
    /// The user can order the items with the 'Space' bar and the arrows. On 'Enter' ordered list of the incides of items will be returned.
    /// The dialog is rendered on stderr.
    /// Result contains `Some(Vec<index>)` if user hit 'Enter' or `None` if user went back with 'Esc' or 'q' (the prompt is erased).
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use paloma::Sort;
    ///
    /// fn main() {
    ///     let items = vec!["foo", "bar", "baz"];
    ///
    ///     let ordered = Sort::new()
    ///         .items(&items)
    ///         .interact_opt()
    ///         .unwrap();
    ///
    ///     match ordered {
    ///         Some(positions) => {
    ///             println!("You prefer:");
    ///
    ///             for i in positions {
    ///                 println!("{}", items[i]);
    ///             }
    ///         },
    ///         None => println!("You did not prefer anything.")
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
            Some(order) => {
                for &i in order {
                    check_index(i, self.items.len())?;
                }
                Some(order.clone())
            }
            // The original order.
            None if options.unattended() => Some((0..self.items.len()).collect()),
            None => None,
        };
        if let Some(given) = given {
            if done != Done::Erase {
                let mut render = TermThemeRenderer::new(term, self.base.theme);
                let prompt = self.prompt.as_deref().unwrap_or("");
                let list: Vec<&str> = given.iter().map(|&i| self.items[i].as_str()).collect();
                render.finish_preset(done, PromptKind::Sort, prompt, Answer::List(&list))?;
            }
            return Ok(Some(given));
        }

        if !term.is_term() {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "not a terminal").into());
        }

        if self.items.is_empty() {
            Err(io::Error::other("Empty list of items given to `Sort`"))?;
        }

        let mut paging = Paging::new(term, self.items.len(), self.max_length);
        let mut render = TermThemeRenderer::new(term, self.base.theme);
        render.set_legend(self.legend_enabled(&options), allow_back);
        // A flow going back to this prompt's step puts its message in the row.
        if let Some(message) = crate::flow::take_pending() {
            self.row.invalid(message);
        }
        let mut sel = 0;

        let mut order: Vec<_> = (0..self.items.len()).collect();
        let mut checked: bool = false;

        term.hide_cursor()?;

        let worker = self.row.worker();
        Keys::run(worker, |keys| loop {
            self.draw(&mut render, &mut paging, sel, &order, checked)?;
            let before = order.clone();

            let Some(key) = keys.next(term, &mut self.row)? else {
                continue;
            };
            match key {
                Key::CtrlC => return render.cancel(),
                Key::ArrowDown | Key::Tab | Key::Char('j') => {
                    let old_sel = sel;

                    sel = (sel + 1) % self.items.len();

                    if checked && old_sel != sel {
                        order.swap(old_sel, sel);
                    }
                }
                Key::ArrowUp | Key::BackTab | Key::Char('k') => {
                    let old_sel = sel;

                    sel = (sel + self.items.len() - 1) % self.items.len();

                    if checked && old_sel != sel {
                        order.swap(old_sel, sel);
                    }
                }
                Key::ArrowLeft | Key::Char('h') => {
                    if paging.active {
                        let old_sel = sel;
                        let old_page = paging.current_page;

                        sel = paging.previous_page();

                        if checked {
                            let indexes: Vec<_> = if old_page == 0 {
                                let indexes1: Vec<_> = (0..=old_sel).rev().collect();
                                let indexes2: Vec<_> = (sel..self.items.len()).rev().collect();
                                [indexes1, indexes2].concat()
                            } else {
                                (sel..=old_sel).rev().collect()
                            };

                            for index in 0..(indexes.len() - 1) {
                                order.swap(indexes[index], indexes[index + 1]);
                            }
                        }
                    }
                }
                Key::ArrowRight | Key::Char('l') => {
                    if paging.active {
                        let old_sel = sel;
                        let old_page = paging.current_page;

                        sel = paging.next_page();

                        if checked {
                            let indexes: Vec<_> = if old_page == paging.pages - 1 {
                                let indexes1: Vec<_> = (old_sel..self.items.len()).collect();
                                let indexes2: Vec<_> = vec![0];
                                [indexes1, indexes2].concat()
                            } else {
                                (old_sel..=sel).collect()
                            };

                            for index in 0..(indexes.len() - 1) {
                                order.swap(indexes[index], indexes[index + 1]);
                            }
                        }
                    }
                }
                Key::Char(' ') => {
                    checked = !checked;
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
                    if let Some(err) = self.validators.check(&order) {
                        self.row.invalid(err);
                        continue;
                    }

                    let list: Vec<_> = order
                        .iter()
                        .map(|item| self.items[*item].as_str())
                        .collect();
                    let prompt = self.prompt.as_deref().unwrap_or("");
                    render.finish(done, PromptKind::Sort, prompt, Answer::List(&list))?;

                    return Ok(Some(order));
                }
                _ => {}
            }

            if order != before {
                self.row.changed(&order);
            }

            paging.update(sel);
        })
    }
}

impl Sort<'_> {
    /// Draws the question, the items of the current page in their current order and
    /// the row.
    fn draw(
        &self,
        render: &mut TermThemeRenderer,
        paging: &mut Paging,
        sel: usize,
        order: &[usize],
        checked: bool,
    ) -> Result {
        let row = self.row.current(self.base.description.as_ref());
        paging.set_reserved(row_lines(row) + render.legend_lines(PromptKind::Sort), sel);
        let items = order
            .iter()
            .enumerate()
            .skip(paging.current_page * paging.capacity)
            .take(paging.capacity)
            .map(|(idx, item)| {
                ItemView::new(
                    PromptKind::Sort,
                    &self.items[*item],
                    sel == idx,
                    Some(checked),
                    None,
                    None,
                    false,
                )
            });
        render.prompt_frame(
            PromptKind::Sort,
            self.prompt.as_deref(),
            paging.counter(),
            self.row.is_invalid(),
            items,
            row,
        )
    }
}

impl<'a> Sort<'a> {
    /// Creates a sort prompt drawn with the app theme (set with [`set_theme`](crate::set_theme), `SimpleTheme` by default).
    ///
    /// [`theme`](Self::theme) gives this prompt a theme of its own.
    pub fn new() -> Self {
        Self {
            items: vec![],
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

    #[test]
    fn test_clone() {
        let sort = Sort::new().prompt("Which order do you prefer?");

        let _ = sort.clone();
    }

    #[test]
    fn test_iterator() {
        let items = ["First", "Second", "Third"];
        let iterator = items.iter().skip(1);

        assert_eq!(Sort::new().items(iterator).items, &items[1..]);
    }

    #[cfg(unix)]
    mod unattended {
        use super::*;
        use crate::{prompts::testing::capture, theme::SimpleTheme};

        #[test]
        fn a_preset_answers_without_a_terminal() {
            let (term, out) = capture();
            let order = Sort::new()
                .theme(&SimpleTheme)
                .prompt("Rank")
                .items(["x", "y", "z"])
                .preset(Some(vec![2, 0, 1]))
                .interact_on(&term)
                .unwrap();
            assert_eq!(order, [2, 0, 1]);
            assert_eq!(out.text(), "Rank: z, x, y\n");
        }

        #[test]
        fn a_preset_draws_as_done_says() {
            for (done, expected) in [
                (Done::Collapse, "Rank: y, x\n"),
                (Done::Keep, "Rank: y, x\n"),
                (Done::Erase, ""),
            ] {
                let (term, out) = capture();
                Sort::new()
                    .theme(&SimpleTheme)
                    .prompt("Rank")
                    .items(["x", "y"])
                    .done(done)
                    .preset(Some(vec![1, 0]))
                    .interact_on(&term)
                    .unwrap();
                assert_eq!(out.text(), expected, "{done:?}");
            }
        }

        #[test]
        fn a_preset_that_names_no_item_is_an_error() {
            let (term, _) = capture();
            let err = Sort::new()
                .items(["x"])
                .preset(Some(vec![1]))
                .interact_on(&term);
            assert!(matches!(err, Err(crate::Error::IO(_))));
        }
    }
}
