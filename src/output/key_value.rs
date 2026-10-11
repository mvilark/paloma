//! Aligned key/value rows.
use std::fmt;

use crate::{
    output::block::{block_methods, BlockBase},
    theme::{BlockKind, Context, Theme},
};

/// What a theme gets to draw key/value rows.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct KeyValueView<'a> {
    /// The rows, in order.
    pub rows: &'a [(String, String)],
}

impl<'a> KeyValueView<'a> {
    /// A view of key/value rows.
    pub fn new(rows: &'a [(String, String)]) -> Self {
        KeyValueView { rows }
    }
}

/// Key/value rows with the values aligned.
///
/// ```rust,no_run
/// use paloma::KeyValue;
///
/// # fn main() -> paloma::Result {
/// KeyValue::new()
///     .row("State", "Connected")
///     .row("Server", "es-bcn-01")
///     .print()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default)]
pub struct KeyValue<'a> {
    rows: Vec<(String, String)>,
    base: BlockBase<'a>,
}

impl<'a> KeyValue<'a> {
    /// No rows yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a row.
    pub fn row(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.rows.push((key.into(), value.into()));
        self
    }

    /// Adds rows.
    pub fn rows<K, V>(mut self, rows: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.rows
            .extend(rows.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    block_methods!(BlockKind::KeyValue);

    fn draw(&self, f: &mut dyn fmt::Write, theme: &dyn Theme, ctx: &Context) -> fmt::Result {
        theme.format_key_value(f, ctx, &KeyValueView::new(&self.rows))
    }
}
