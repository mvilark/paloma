//! A table with a header row.
use std::fmt;

use crate::{
    output::block::{block_methods, BlockBase},
    theme::{BlockKind, Context, Theme},
};

/// What a theme gets to draw a table.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct TableView<'a> {
    /// The header cells (may be empty for a table without a header).
    pub header: &'a [String],
    /// The rows of cells.
    pub rows: &'a [Vec<String>],
}

impl<'a> TableView<'a> {
    /// A view of a table.
    pub fn new(header: &'a [String], rows: &'a [Vec<String>]) -> Self {
        TableView { header, rows }
    }
}

/// A table: columns padded to their widest cell, a header in its own style.
///
/// ```rust,no_run
/// use paloma::Table;
///
/// # fn main() -> paloma::Result {
/// Table::new(["Server", "Load"])
///     .row(["us-nyc-01", "42%"])
///     .row(["es-bcn-01", "23%"])
///     .print()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default)]
pub struct Table<'a> {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
    base: BlockBase<'a>,
}

impl<'a> Table<'a> {
    /// A table with this header row (pass an empty list for no header).
    pub fn new<S: Into<String>>(header: impl IntoIterator<Item = S>) -> Self {
        Table {
            header: header.into_iter().map(Into::into).collect(),
            rows: Vec::new(),
            base: BlockBase::default(),
        }
    }

    /// Adds a row.
    pub fn row<S: Into<String>>(mut self, cells: impl IntoIterator<Item = S>) -> Self {
        self.rows.push(cells.into_iter().map(Into::into).collect());
        self
    }

    /// Adds rows.
    pub fn rows<R, S>(mut self, rows: impl IntoIterator<Item = R>) -> Self
    where
        R: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for row in rows {
            self = self.row(row);
        }
        self
    }

    block_methods!(BlockKind::Table);

    fn draw(&self, f: &mut dyn fmt::Write, theme: &dyn Theme, ctx: &Context) -> fmt::Result {
        theme.format_table(f, ctx, &TableView::new(&self.header, &self.rows))
    }
}
