//! paloma is a UI framework for command-line programs that run one action and
//! exit: prompts, flows of prompts, messages, loading messages and progress bars,
//! all drawn through one app-wide [`Theme`](theme::Theme) and configured by app-wide
//! [`Options`]. Every component has a sensible default, so it works with minimal
//! code, and every look and behaviour can be changed with paloma's own tools.
//! It is line-oriented: there are no full-screen views.
//!
//! The prompts are built; flows, messages, spinners and progress bars are on the
//! roadmap. paloma draws with [console](https://docs.rs/console), re-exported as
//! [`paloma::console`](console).
//!
//! # Crate Contents
//!
//! * Confirmation prompts
//! * Input prompts (regular and password)
//! * Input validation
//! * Selections prompts (single and multi)
//! * Fuzzy select prompt
//! * Other kind of prompts
//! * Editor launching
//!
//! # Crate Features
//!
//! The following crate features are available:
//! * `editor`: enables bindings to launch editor to edit strings
//! * `fuzzy-select`: enables fuzzy select prompt
//! * `history`: enables input prompts to be able to track history of inputs
//! * `password`: enables password input prompt
//! * `completion`: enables ability to implement custom tab-completion for input prompts
//!
//! By default `editor` and `password` are enabled.

#![deny(clippy::all, missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub use console;

#[cfg(feature = "completion")]
pub use completion::Completion;
#[cfg(feature = "editor")]
pub use edit::Editor;
pub use error::{Error, Result};
#[cfg(feature = "history")]
pub use history::{BasicHistory, History};
pub use options::{set_options, Options};
pub use theme::app::set_theme;
pub use validate::{InputValidator, PasswordValidator};

#[cfg(feature = "fuzzy-select")]
pub use prompts::fuzzy_select::FuzzySelect;
#[cfg(feature = "password")]
pub use prompts::password::Password;
pub use prompts::{
    confirm::Confirm, input::Input, multi_select::MultiSelect, select::Select, sort::Sort,
};

#[cfg(feature = "completion")]
mod completion;
#[cfg(feature = "editor")]
mod edit;
mod error;
#[cfg(feature = "history")]
mod history;
pub mod options;
mod paging;
mod prompts;
pub mod theme;
mod validate;
