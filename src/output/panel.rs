//! A titled box of rows, such as a summary at the end of a command.
use std::fmt;

use crate::{
    output::{
        block::{block_methods, BlockBase},
        message::{Message, Status},
    },
    theme::{BlockKind, Context, Theme},
};

/// One row of a [`Panel`]: text, an optional status glyph before it and an optional
/// right-aligned value.
#[derive(Clone, Debug)]
pub struct PanelRow {
    status: Option<Status>,
    text: String,
    trailing: Option<String>,
}

impl PanelRow {
    /// A row with this text.
    pub fn new(text: impl Into<String>) -> Self {
        PanelRow {
            status: None,
            text: text.into(),
            trailing: None,
        }
    }

    /// Draws `status`'s message glyph before the text.
    pub fn status(mut self, status: Status) -> Self {
        self.status = Some(status);
        self
    }

    /// A value drawn right-aligned, such as a duration.
    pub fn trailing(mut self, text: impl Into<String>) -> Self {
        self.trailing = Some(text.into());
        self
    }

    /// The row's status, if any.
    pub fn get_status(&self) -> Option<Status> {
        self.status
    }

    /// The row's text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The right-aligned value, if any.
    pub fn get_trailing(&self) -> Option<&str> {
        self.trailing.as_deref()
    }
}

impl From<&str> for PanelRow {
    fn from(text: &str) -> Self {
        PanelRow::new(text)
    }
}

impl From<String> for PanelRow {
    fn from(text: String) -> Self {
        PanelRow::new(text)
    }
}

/// A message becomes a row with its status and text (nested messages are dropped).
impl From<Message<'_>> for PanelRow {
    fn from(message: Message<'_>) -> Self {
        PanelRow::new(message.text()).status(message.status())
    }
}

/// What a theme gets to draw a panel.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct PanelView<'a> {
    /// The title.
    pub title: &'a str,
    /// The rows.
    pub rows: &'a [PanelRow],
}

impl<'a> PanelView<'a> {
    /// A view of a panel.
    pub fn new(title: &'a str, rows: &'a [PanelRow]) -> Self {
        PanelView { title, rows }
    }
}

/// A box with a title and rows.
///
/// ```rust,no_run
/// use paloma::{Message, Panel, PanelRow};
///
/// # fn main() -> paloma::Result {
/// Panel::new("Setup complete")
///     .row(PanelRow::from(Message::success("Signed in")).trailing("1.8s"))
///     .row("Connect on startup: On")
///     .print()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Panel<'a> {
    title: String,
    rows: Vec<PanelRow>,
    base: BlockBase<'a>,
}

impl<'a> Panel<'a> {
    /// A panel with this title.
    pub fn new(title: impl Into<String>) -> Self {
        Panel {
            title: title.into(),
            rows: Vec::new(),
            base: BlockBase::default(),
        }
    }

    /// Adds a row: text, a [`PanelRow`] or a [`Message`].
    pub fn row(mut self, row: impl Into<PanelRow>) -> Self {
        self.rows.push(row.into());
        self
    }

    /// Adds rows.
    pub fn rows<R: Into<PanelRow>>(mut self, rows: impl IntoIterator<Item = R>) -> Self {
        self.rows.extend(rows.into_iter().map(Into::into));
        self
    }

    block_methods!(BlockKind::Panel);

    fn draw(&self, f: &mut dyn fmt::Write, theme: &dyn Theme, ctx: &Context) -> fmt::Result {
        theme.format_panel(f, ctx, &PanelView::new(&self.title, &self.rows))
    }
}
