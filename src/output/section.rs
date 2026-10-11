//! A section title.
use std::fmt;

use crate::{
    output::block::{block_methods, BlockBase},
    theme::{BlockKind, Context, Theme},
};

/// What a theme gets to draw a section title.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct SectionView<'a> {
    /// The title.
    pub title: &'a str,
}

impl<'a> SectionView<'a> {
    /// A view of a section title.
    pub fn new(title: &'a str) -> Self {
        SectionView { title }
    }
}

/// A section title, such as `# Setup`.
///
/// ```rust,no_run
/// # fn main() -> paloma::Result {
/// paloma::section("Setup").print()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Section<'a> {
    title: String,
    base: BlockBase<'a>,
}

/// A section title; see [`Section`].
pub fn section(title: impl Into<String>) -> Section<'static> {
    Section::new(title)
}

impl<'a> Section<'a> {
    /// A section with this title.
    pub fn new(title: impl Into<String>) -> Self {
        Section {
            title: title.into(),
            base: BlockBase::default(),
        }
    }

    block_methods!(BlockKind::Section);

    fn draw(&self, f: &mut dyn fmt::Write, theme: &dyn Theme, ctx: &Context) -> fmt::Result {
        theme.format_section(f, ctx, &SectionView::new(&self.title))
    }
}
