//! What every output block shares: its own theme and stream, and how it is printed.
use std::fmt;

use console::Term;

use crate::{
    terminal::Stream,
    theme::{app, BlockKind, Context, Theme},
    Result,
};

/// State every output block holds. Unset fields fall back to the app theme and the
/// app options.
#[derive(Clone, Default)]
pub(crate) struct BlockBase<'a> {
    pub(crate) theme: Option<&'a dyn Theme>,
    pub(crate) stream: Option<Stream>,
}

impl BlockBase<'_> {
    /// The stream from the block, its member group, the `blocks` family, then the
    /// block's built-in default.
    pub(crate) fn stream(&self, kind: BlockKind) -> Stream {
        let options = crate::options::current();
        self.stream
            .or(options.block(kind).stream)
            .or(options.blocks.stream)
            .unwrap_or(match kind {
                BlockKind::KeyValue | BlockKind::Table => Stream::Stdout,
                _ => Stream::Stderr,
            })
    }
}

/// Draws with `theme` (or the app theme) for `stream` and writes the result to `term`
/// as whole lines, after `space_before` blank lines when `kind` is a block.
pub(crate) fn emit(
    term: &Term,
    stream: Stream,
    theme: Option<&dyn Theme>,
    kind: Option<BlockKind>,
    draw: impl FnOnce(&mut dyn fmt::Write, &dyn Theme, &Context) -> fmt::Result,
) -> Result {
    let app_theme;
    let theme: &dyn Theme = match theme {
        Some(theme) => theme,
        None => {
            app_theme = app::current();
            &*app_theme
        }
    };
    let tokens = theme.tokens();
    let ctx = Context::new(&tokens, stream);
    let mut out = String::new();
    if let Some(kind) = kind {
        let space = tokens.get_blocks().resolve(kind, &tokens).space_before;
        out.push_str(&"\n".repeat(space));
    }
    draw(&mut out, theme, &ctx).map_err(std::io::Error::other)?;
    term.write_line(&out)?;
    term.flush()?;
    crate::flow::record_on(term, &out);
    Ok(())
}

/// Adds the builder methods every block shares (`theme`, `stream`, `print`,
/// `print_on`) to a block with a `base: BlockBase` field and a `draw` method. Use it
/// inside an `impl<'a> Block<'a>` block.
macro_rules! block_methods {
    ($kind:expr) => {
        /// Draws this block with `theme` instead of the app theme set with
        /// [`set_theme`](crate::set_theme).
        pub fn theme(mut self, theme: &'a dyn crate::theme::Theme) -> Self {
            self.base.theme = Some(theme);
            self
        }

        /// Where [`print`](Self::print) writes. Left unset, the app options decide
        /// ([`Options::blocks`](crate::Options::blocks) and this block's group), then
        /// the block's default.
        pub fn stream(mut self, stream: crate::terminal::Stream) -> Self {
            self.base.stream = Some(stream);
            self
        }

        /// Prints the block.
        pub fn print(self) -> crate::Result {
            let stream = self.base.stream($kind);
            self.print_on(&stream.term())
        }

        /// Like [`print`](Self::print) but writes to `term`.
        pub fn print_on(self, term: &console::Term) -> crate::Result {
            let stream = self.base.stream($kind);
            crate::output::block::emit(
                term,
                stream,
                self.base.theme,
                Some($kind),
                |f, theme, ctx| self.draw(f, theme, ctx),
            )
        }
    };
}
pub(crate) use block_methods;
