//! Components that redraw while work runs, then leave a static result line.
//!
//! [`engine`] owns the one live region on screen (the redraw thread, the hidden
//! cursor and the pause a prompt asks for); [`task`] is the spinner built on it.
pub(crate) mod engine;
pub(crate) mod task;
