use std::{io, ops::Rem};

use console::{Key, Term};

use crate::{
    output::message::{IntoMessage, Message, Status},
    paging::Paging,
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
    default: usize,
    items: Vec<String>,
    prompt: Option<String>,
    clear: bool,
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
    /// Indicates whether select menu should be erased from the screen after interaction.
    ///
    /// The default is to clear the menu.
    pub fn clear(mut self, val: bool) -> Self {
        self.clear = val;
        self
    }

    /// Sets initial selected element when select menu is rendered
    ///
    /// Element is indicated by the index at which it appears in [`item`](Self::item) method invocation or [`items`](Self::items) slice.
    pub fn default(mut self, val: usize) -> Self {
        self.default = val;
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

    /// Add a single item to the selector.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use paloma::Select;
    ///
    /// fn main() {
    ///     let selection = Select::new()
    ///         .item("Item 1")
    ///         .item("Item 2")
    ///         .interact()
    ///         .unwrap();
    /// }
    /// ```
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

    /// Sets the select prompt.
    ///
    /// By default, when a prompt is set the system also prints out a confirmation after
    /// the selection. You can opt-out of this with [`report`](Self::report).
    pub fn prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    base_methods!(select, |s: &Self| s.prompt.is_some());

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
    /// This unlike [`interact_opt`](Self::interact_opt) does not allow to quit with 'Esc' or 'q'.
    #[inline]
    pub fn interact(self) -> Result<usize> {
        self.interact_on(&Term::stderr())
    }

    /// Enables user interaction and returns the result.
    ///
    /// The user can select the items with the 'Space' bar or 'Enter' and the index of selected item will be returned.
    /// The dialog is rendered on stderr.
    /// Result contains `Some(index)` if user selected one of items using 'Enter' or `None` if user cancelled with 'Esc' or 'q'.
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
        Ok(self
            ._interact_on(term, false)?
            .ok_or_else(|| io::Error::other("Quit not allowed in this case"))?)
    }

    /// Like [`interact_opt`](Self::interact_opt) but allows a specific terminal to be set.
    #[inline]
    pub fn interact_on_opt(self, term: &Term) -> Result<Option<usize>> {
        self._interact_on(term, true)
    }

    /// Like `interact` but allows a specific terminal to be set.
    fn _interact_on(mut self, term: &Term, allow_quit: bool) -> Result<Option<usize>> {
        if !term.is_term() {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "not a terminal").into());
        }

        if self.items.is_empty() {
            Err(io::Error::other("Empty list of items given to `Select`"))?;
        }

        let mut paging = Paging::new(term, self.items.len(), self.max_length);
        let mut render = TermThemeRenderer::new(term, self.base.theme);
        let mut sel = self.default;

        term.hide_cursor()?;
        paging.update_page(sel);

        loop {
            self.draw(&mut render, &mut paging, sel)?;
            let highlighted = sel;

            match term.read_key()? {
                Key::ArrowDown | Key::Tab | Key::Char('j') => {
                    if sel == !0 {
                        sel = 0;
                    } else {
                        sel = (sel as u64 + 1).rem(self.items.len() as u64) as usize;
                    }
                }
                Key::Escape | Key::Char('q') => {
                    if allow_quit {
                        if self.clear {
                            render.erase()?;
                        } else {
                            render.keep_question(
                                PromptKind::Select,
                                self.prompt.as_deref(),
                                paging.counter(),
                                self.row.is_invalid(),
                            )?;
                        }

                        term.show_cursor()?;
                        term.flush()?;

                        return Ok(None);
                    }
                }
                Key::ArrowUp | Key::BackTab | Key::Char('k') => {
                    if sel == !0 {
                        sel = self.items.len() - 1;
                    } else {
                        sel = ((sel as i64 - 1 + self.items.len() as i64)
                            % (self.items.len() as i64)) as usize;
                    }
                }
                Key::ArrowLeft | Key::Char('h') => {
                    if paging.active {
                        sel = paging.previous_page();
                    }
                }
                Key::ArrowRight | Key::Char('l') => {
                    if paging.active {
                        sel = paging.next_page();
                    }
                }

                Key::Enter | Key::Char(' ') if sel != !0 => {
                    if let Some(err) = self.validators.check(&sel) {
                        self.row.invalid(err);
                        continue;
                    }

                    if self.clear {
                        render.erase()?;
                    } else {
                        render.commit()?;
                    }

                    if let Some(ref prompt) = self.prompt {
                        if self.report_enabled() {
                            render.answered(
                                PromptKind::Select,
                                prompt,
                                Answer::Text(&self.items[sel]),
                            )?;
                        }
                    }

                    term.show_cursor()?;
                    term.flush()?;

                    return Ok(Some(sel));
                }
                _ => {}
            }

            if sel != highlighted && sel != !0 {
                self.row.changed(&sel);
            }

            paging.update(sel);
        }
    }

    /// Draws the question, the items of the current page and the row.
    fn draw(&self, render: &mut TermThemeRenderer, paging: &mut Paging, sel: usize) -> Result {
        let row = self.row.current(self.base.description.as_ref());
        paging.set_reserved(row_lines(row), sel);
        let items = self
            .items
            .iter()
            .enumerate()
            .skip(paging.current_page * paging.capacity)
            .take(paging.capacity)
            .map(|(idx, item)| ItemView::new(PromptKind::Select, item, sel == idx, None, None));
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
            default: !0,
            items: vec![],
            prompt: None,
            clear: true,
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
            Select::new().default(0).items(&selections[..]).items,
            selections
        );
    }

    #[test]
    fn test_string() {
        let selections = vec!["a".to_string(), "b".to_string()];

        assert_eq!(
            Select::new().default(0).items(&selections).items,
            selections
        );
    }

    #[test]
    fn test_ref_str() {
        let a = "a";
        let b = "b";

        let selections = &[a, b];

        assert_eq!(Select::new().default(0).items(selections).items, selections);
    }

    #[test]
    fn test_iterator() {
        let items = ["First", "Second", "Third"];
        let iterator = items.iter().skip(1);

        assert_eq!(Select::new().default(0).items(iterator).items, &items[1..]);
    }
}
