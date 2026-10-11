use std::io;

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
    Error, Result,
};

/// Renders a select prompt.
///
/// User can select from one or more options.
/// Interaction returns index of an item selected in the order they appear in `item` invocation or `items` slice.
///
/// ## Example
///
/// ```rust,no_run
/// use paloma::Select;
///
/// fn main() {
///     let items = vec!["foo", "bar", "baz"];
///
///     let selection = Select::new()
///         .prompt("What do you choose?")
///         .items(&items)
///         .interact()
///         .unwrap();
///
///     println!("You chose: {}", items[selection]);
/// }
/// ```
#[derive(Clone)]
pub struct Select<'a> {
    default: Option<usize>,
    items: Vec<Item>,
    prompt: Option<String>,
    preset: Option<usize>,
    base: PromptBase<'a>,
    max_length: Option<usize>,
    validators: Validators<'a, usize>,
    row: Row<'a, usize>,
}

impl Default for Select<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Select<'a> {
    /// Sets initial selected element when select menu is rendered
    ///
    /// Element is indicated by the index at which it appears in [`item`](Self::item) method invocation or [`items`](Self::items) slice.
    /// A disabled element starts on the next enabled one. With
    /// `general().unattended(true)` the prompt answers with this element.
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

    /// Sets an optional max length for a page.
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
    /// disabled) or any `Display` value.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use paloma::{Item, Select};
    ///
    /// fn main() {
    ///     let selection = Select::new()
    ///         .item("Item 1")
    ///         .item(Item::new("Item 2").description("not yet").disabled(true))
    ///         .interact()
    ///         .unwrap();
    /// }
    /// ```
    pub fn item(mut self, item: impl IntoItem) -> Self {
        self.items.push(item.into_item());

        self
    }

    /// Adds multiple items to the selector (see [`item`](Self::item)).
    pub fn items<T, I>(mut self, items: I) -> Self
    where
        T: IntoItem,
        I: IntoIterator<Item = T>,
    {
        self.items
            .extend(items.into_iter().map(IntoItem::into_item));

        self
    }

    /// Sets the select prompt.
    ///
    /// By default, when a prompt is set the answer is left as one line after the
    /// selection. Change that with [`done`](Self::done).
    pub fn prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    base_methods!(select);

    /// Called each time the highlighted item changes, with its index (only once an
    /// item is highlighted). `Some(message)` shows in the row under the items; `None`
    /// brings the [`description`](Self::description) back.
    ///
    /// A later call replaces an earlier closure.
    pub fn on_change(
        mut self,
        f: impl FnMut(&usize) -> Option<Message<'static>> + Send + 'a,
    ) -> Self {
        self.row.on_change(f);
        self
    }

    /// Registers a validator, run with the chosen index when the user picks an item.
    /// An error keeps the prompt open and shows in the row under the items until the
    /// highlight moves. Several validators run in order; the first error wins.
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

    /// Enables user interaction and returns the result.
    ///
    /// The user can select the items with the 'Space' bar or 'Enter' and the index of selected item will be returned.
    /// The dialog is rendered on stderr.
    /// Result contains `index` if user selected one of items using 'Enter'.
    /// This unlike [`interact_opt`](Self::interact_opt) does not allow to quit with 'Esc' or 'q',
    /// except inside a [`Flow`](crate::Flow), where `Esc` or `q` returns [`Error::Back`] (the step goes back).
    /// `Ctrl+C` returns [`Error::Cancelled`].
    #[inline]
    pub fn interact(self) -> Result<usize> {
        self.interact_on(&Term::stderr())
    }

    /// Enables user interaction and returns the result.
    ///
    /// The user can select the items with the 'Space' bar or 'Enter' and the index of selected item will be returned.
    /// The dialog is rendered on stderr.
    /// Result contains `Some(index)` if user selected one of items using 'Enter' or `None` if user went back with 'Esc' or 'q' (the prompt is erased).
    ///
    /// ## Example
    ///
    ///```rust,no_run
    /// use paloma::Select;
    ///
    /// fn main() {
    ///     let items = vec!["foo", "bar", "baz"];
    ///
    ///     let selection = Select::new()
    ///         .prompt("What do you choose?")
    ///         .items(&items)
    ///         .interact_opt()
    ///         .unwrap();
    ///
    ///     match selection {
    ///         Some(index) => println!("You chose: {}", items[index]),
    ///         None => println!("You did not choose anything.")
    ///     }
    /// }
    ///```
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

    /// Like `interact` but allows a specific terminal to be set.
    fn _interact_on(mut self, term: &Term, allow_back: bool) -> Result<Option<usize>> {
        let options = crate::options::current();
        let done = match self.done_mode(&options) {
            Done::Collapse if self.prompt.is_none() => Done::Erase,
            done => done,
        };

        // A preset or an unattended run answers without a terminal.
        let given = match self.preset {
            Some(sel) => Some(check_index(sel, self.items.len())?),
            None if options.unattended() => Some(self.unattended()?),
            None => None,
        };
        if let Some(sel) = given {
            if done != Done::Erase {
                let mut render = TermThemeRenderer::new(term, self.base.theme);
                let prompt = self.prompt.as_deref().unwrap_or("");
                let answer = Answer::Text(self.items[sel].text());
                render.finish_preset(done, PromptKind::Select, prompt, answer)?;
            }
            return Ok(Some(sel));
        }

        if !term.is_term() {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "not a terminal").into());
        }

        if self.items.is_empty() {
            Err(io::Error::other("Empty list of items given to `Select`"))?;
        }

        let mut paging = Paging::new(term, self.items.len(), self.max_length);
        let mut render = TermThemeRenderer::new(term, self.base.theme);
        render.set_legend(self.legend_enabled(&options), allow_back);
        // A flow going back to this prompt's step puts its message in the row.
        if let Some(message) = crate::flow::take_pending() {
            self.row.invalid(message);
        }
        // The highlighted item, or `!0` before one is. A disabled default starts on the
        // next enabled item.
        let mut sel = self
            .default
            .filter(|&d| d < self.items.len())
            .and_then(|d| next_enabled(&self.items, d, true))
            .unwrap_or(!0);

        term.hide_cursor()?;
        paging.update_page(sel);

        let worker = self.row.worker();
        Keys::run(worker, |keys| loop {
            self.draw(&mut render, &mut paging, sel)?;
            let highlighted = sel;

            let Some(key) = keys.next(term, &mut self.row)? else {
                continue;
            };
            match key {
                Key::CtrlC => return render.cancel(),
                Key::ArrowDown | Key::Tab | Key::Char('j') => {
                    sel = step_enabled(&self.items, sel, true).unwrap_or(sel);
                }
                Key::Escape | Key::Char('q') => {
                    if allow_back {
                        render.back()?;
                        return Ok(None);
                    }
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

                Key::Enter | Key::Char(' ') if sel != !0 => {
                    let validates = !self.validators.is_empty();
                    if keys.defer_submit(&mut self.row, validates) {
                        continue;
                    }
                    if let Some(err) = self.validators.check(&sel) {
                        self.row.invalid(err);
                        continue;
                    }

                    let answer = Answer::Text(self.items[sel].text());
                    render.finish(
                        done,
                        PromptKind::Select,
                        self.prompt.as_deref().unwrap_or(""),
                        answer,
                    )?;

                    return Ok(Some(sel));
                }
                _ => {}
            }

            if sel != highlighted && sel != !0 {
                self.row.changed(&sel);
            }

            paging.update(sel);
        })
    }

    /// The index an unattended run answers with: the default, or the next enabled item
    /// when that one is disabled.
    fn unattended(&self) -> Result<usize> {
        let default = self.default.ok_or(Error::NoDefault)?;
        let default = check_index(default, self.items.len())?;
        next_enabled(&self.items, default, true).ok_or(Error::NoDefault)
    }

    /// Draws the question, the items of the current page and the row.
    fn draw(&self, render: &mut TermThemeRenderer, paging: &mut Paging, sel: usize) -> Result {
        let row = self.row.current(self.base.description.as_ref());
        paging.set_reserved(
            row_lines(row) + render.legend_lines(PromptKind::Select),
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
                    PromptKind::Select,
                    item.text(),
                    sel == idx,
                    None,
                    None,
                    item.description_text(),
                    item.is_disabled(),
                )
            });
        render.prompt_frame(
            PromptKind::Select,
            self.prompt.as_deref(),
            paging.counter(),
            self.row.is_invalid(),
            items,
            row,
        )
    }
}

impl<'a> Select<'a> {
    /// Creates a select prompt drawn with the app theme (set with [`set_theme`](crate::set_theme), `SimpleTheme` by default).
    ///
    /// [`theme`](Self::theme) gives this prompt a theme of its own.
    pub fn new() -> Self {
        Self {
            default: None,
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

    fn texts(select: &Select) -> Vec<String> {
        select.items.iter().map(|i| i.text().to_string()).collect()
    }

    #[test]
    fn test_clone() {
        let select = Select::new().prompt("Do you want to continue?");

        let _ = select.clone();
    }

    #[test]
    fn test_str() {
        let selections = &[
            "Ice Cream",
            "Vanilla Cupcake",
            "Chocolate Muffin",
            "A Pile of sweet, sweet mustard",
        ];

        assert_eq!(
            texts(&Select::new().default(0).items(&selections[..])),
            selections
        );
    }

    #[test]
    fn test_string() {
        let selections = vec!["a".to_string(), "b".to_string()];

        assert_eq!(
            texts(&Select::new().default(0).items(&selections)),
            selections
        );
    }

    #[test]
    fn test_ref_str() {
        let a = "a";
        let b = "b";

        let selections = &[a, b];

        assert_eq!(
            texts(&Select::new().default(0).items(selections)),
            selections
        );
    }

    #[test]
    fn test_iterator() {
        let items = ["First", "Second", "Third"];
        let iterator = items.iter().skip(1);

        assert_eq!(
            texts(&Select::new().default(0).items(iterator)),
            &items[1..]
        );
    }

    #[test]
    fn items_can_be_rich() {
        let select = Select::new()
            .item("a")
            .item(Item::new("b").description("more").disabled(true))
            .items([1, 2]);
        assert_eq!(texts(&select), ["a", "b", "1", "2"]);
        assert!(select.items[1].is_disabled());
        assert_eq!(select.items[1].description_text(), Some("more"));
    }

    #[cfg(unix)]
    mod unattended {
        use super::*;
        use crate::{prompts::testing::capture, theme::SimpleTheme};

        #[test]
        fn a_preset_answers_without_a_terminal() {
            let (term, out) = capture();
            let picked = Select::new()
                .theme(&SimpleTheme)
                .prompt("Pick")
                .items(["a", "b"])
                .preset(Some(1))
                .interact_on(&term)
                .unwrap();
            assert_eq!(picked, 1);
            assert_eq!(out.text(), "Pick: b\n");
        }

        #[test]
        fn a_preset_draws_as_done_says() {
            for (done, expected) in [
                (Done::Collapse, "Pick: a\n"),
                (Done::Keep, "Pick: a\n"),
                (Done::Erase, ""),
            ] {
                let (term, out) = capture();
                let picked = Select::new()
                    .theme(&SimpleTheme)
                    .prompt("Pick")
                    .items(["a", "b"])
                    .done(done)
                    .preset(Some(0))
                    .interact_on_opt(&term)
                    .unwrap();
                assert_eq!(picked, Some(0));
                assert_eq!(out.text(), expected, "{done:?}");
            }
        }

        #[test]
        fn a_preset_without_a_question_draws_nothing() {
            let (term, out) = capture();
            let picked = Select::new()
                .theme(&SimpleTheme)
                .items(["a", "b"])
                .preset(Some(1))
                .interact_on(&term)
                .unwrap();
            assert_eq!((picked, out.text().as_str()), (1, ""));
        }

        #[test]
        fn a_preset_that_names_no_item_is_an_error() {
            let (term, _) = capture();
            let err = Select::new()
                .items(["a"])
                .preset(Some(1))
                .interact_on(&term);
            assert!(matches!(err, Err(Error::IO(_))));
        }

        #[test]
        fn none_asks_as_usual_and_a_non_terminal_is_an_error() {
            let (term, out) = capture();
            let err = Select::new().items(["a"]).preset(None).interact_on(&term);
            assert!(matches!(err, Err(Error::IO(_))));
            assert_eq!(out.text(), "");
        }

        #[test]
        fn the_unattended_pick_is_the_default_or_the_next_enabled_item() {
            let select = Select::new().items(["a", "b"]).default(1);
            assert_eq!(select.unattended().unwrap(), 1);
            let select = Select::new()
                .item(Item::new("a"))
                .item(Item::new("b").disabled(true))
                .item(Item::new("c"))
                .default(1);
            assert_eq!(select.unattended().unwrap(), 2);
            let select = Select::new().item(Item::new("a").disabled(true)).default(0);
            assert!(matches!(select.unattended(), Err(Error::NoDefault)));
            // No default: no answer.
            let select = Select::new().items(["a", "b"]);
            assert!(matches!(select.unattended(), Err(Error::NoDefault)));
        }
    }
}
