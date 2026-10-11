use console::Term;

/// Creates a paging module
///
/// The paging module serves as tracking structure to allow paged views
/// and automatically (de-)activates paging depending on the current terminal size.
pub struct Paging<'a> {
    pub pages: usize,
    pub current_page: usize,
    pub capacity: usize,
    pub active: bool,
    pub max_capacity: Option<usize>,
    term: &'a Term,
    current_term_size: (u16, u16),
    items_len: usize,
    /// Rows the message row under the prompt takes, kept free for it.
    reserved: usize,
}

impl<'a> Paging<'a> {
    pub fn new(term: &'a Term, items_len: usize, max_capacity: Option<usize>) -> Paging<'a> {
        let mut paging = Paging {
            pages: 0,
            current_page: 0,
            capacity: 1,
            active: false,
            term,
            current_term_size: term.size(),
            items_len,
            max_capacity,
            reserved: 0,
        };
        paging.resize();
        paging
    }

    /// Recomputes the capacity and the number of pages for the terminal size and the
    /// reserved rows.
    fn resize(&mut self) {
        // Subtract 2 because we need space to render the prompt, if paging is active
        self.capacity = (self
            .max_capacity
            .unwrap_or(usize::MAX)
            .clamp(3, self.current_term_size.0 as usize)
            - 2)
        .saturating_sub(self.reserved)
        .max(1);
        self.pages = self.items_len.div_ceil(self.capacity);
        self.active = self.pages > 1;
    }

    /// Keeps `rows` rows free under the items for the message row (its line count),
    /// so the whole prompt fits on the screen. Call it before each frame.
    ///
    /// The reservation only grows: once a taller row has shown, the pages keep their
    /// size, so page boundaries do not move under the user while the row changes.
    pub fn set_reserved(&mut self, rows: usize, cursor_pos: usize) {
        if rows > self.reserved {
            self.reserved = rows;
            self.resize();
            self.update_page(cursor_pos);
        }
    }

    pub fn update_page(&mut self, cursor_pos: usize) {
        if cursor_pos != !0
            && (cursor_pos < self.current_page * self.capacity
                || cursor_pos >= (self.current_page + 1) * self.capacity)
        {
            self.current_page = cursor_pos / self.capacity;
        }
    }

    /// Updates all internal based on the current terminal size and cursor position
    pub fn update(&mut self, cursor_pos: usize) {
        let new_term_size = self.term.size();

        if self.current_term_size != new_term_size {
            self.current_term_size = new_term_size;
            self.resize();
        }

        self.update_page(cursor_pos);
    }

    /// The paging counter for the question line: the current page and the number of
    /// pages, while paging is active.
    pub fn counter(&self) -> Option<(usize, usize)> {
        self.active.then_some((self.current_page + 1, self.pages))
    }

    /// Navigates to the next page
    pub fn next_page(&mut self) -> usize {
        if self.current_page == self.pages - 1 {
            self.current_page = 0;
        } else {
            self.current_page += 1;
        }

        self.current_page * self.capacity
    }

    /// Navigates to the previous page
    pub fn previous_page(&mut self) -> usize {
        if self.current_page == 0 {
            self.current_page = self.pages - 1;
        } else {
            self.current_page -= 1;
        }

        self.current_page * self.capacity
    }
}
