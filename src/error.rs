use std::{fmt, io::Error as IoError, result::Result as StdResult};

use crate::flow::BackTo;

/// Possible errors returned by prompts.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error while executing IO operations.
    IO(IoError),
    /// The user pressed Ctrl+C. Exit with code 130 to follow the shell convention.
    ///
    /// Prompts read `^C` as a key (paloma installs no signal handler), erase
    /// themselves, draw the theme's cancelled line when it has one, give the cursor
    /// back and return this error, from `interact` and from `interact_opt`.
    Cancelled,
    /// Unattended mode (`Options::general(|g| g.unattended(true))`) asked a prompt that
    /// has no default and no preset.
    NoDefault,
    /// `Esc` in a prompt inside a [`Flow`](crate::Flow): the step goes back.
    /// [`Flow::run`](crate::Flow::run) handles it; outside a flow it never occurs.
    Back,
    /// [`flow::back_to`](crate::flow::back_to): jump to an earlier step.
    /// [`Flow::run`](crate::Flow::run) handles it.
    BackTo(BackTo),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IO(io) => write!(f, "IO error: {}", io),
            Self::Cancelled => write!(f, "cancelled"),
            Self::NoDefault => write!(f, "no default for an unattended prompt"),
            Self::Back => write!(f, "back"),
            Self::BackTo(to) => write!(f, "back to step {}", to.step()),
        }
    }
}

impl std::error::Error for Error {}

impl From<IoError> for Error {
    fn from(err: IoError) -> Self {
        Self::IO(err)
    }
}

/// Result type where errors are of type [Error](enum@Error).
pub type Result<T = ()> = StdResult<T, Error>;

impl From<Error> for IoError {
    /// `Cancelled` becomes `ErrorKind::Interrupted`, `NoDefault`
    /// `ErrorKind::InvalidInput`, and `Back` and `BackTo` `ErrorKind::Other`.
    fn from(value: Error) -> Self {
        match value {
            Error::IO(err) => err,
            Error::Cancelled => IoError::new(std::io::ErrorKind::Interrupted, value),
            Error::NoDefault => IoError::new(std::io::ErrorKind::InvalidInput, value),
            Error::Back | Error::BackTo(_) => IoError::other(value.to_string()),
        }
    }
}

/// The user cancelled (`Ctrl+C` in a prompt, or `Esc` in a flow with no step to go
/// back to), and whatever was live is already erased: prints the app's cancel message
/// on its own line, if it has one, then exits with code 130 or returns
/// [`Error::Cancelled`], per [`GeneralOptions::cancel_exits`](crate::options::GeneralOptions::cancel_exits).
pub(crate) fn cancelled<T>() -> Result<T> {
    let options = crate::options::current();
    if let Some(message) = options.general.cancel_message.clone() {
        let _ = message.into_message().print();
    }
    if options.cancel_exits() {
        crate::terminal::restore();
        std::process::exit(130);
    }
    Err(Error::Cancelled)
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use super::*;

    #[test]
    fn errors_display_and_convert_to_io_errors() {
        assert_eq!(Error::Cancelled.to_string(), "cancelled");
        assert_eq!(
            Error::NoDefault.to_string(),
            "no default for an unattended prompt"
        );
        assert_eq!(
            Error::IO(IoError::other("boom")).to_string(),
            "IO error: boom"
        );
        let kind = |e: Error| IoError::from(e).kind();
        assert_eq!(kind(Error::Cancelled), ErrorKind::Interrupted);
        assert_eq!(kind(Error::NoDefault), ErrorKind::InvalidInput);
        assert_eq!(kind(Error::IO(IoError::other("x"))), ErrorKind::Other);
        assert_eq!(Error::Back.to_string(), "back");
        assert_eq!(kind(Error::Back), ErrorKind::Other);
        let to = crate::flow::back_to::<()>("email", None).unwrap_err();
        assert_eq!(to.to_string(), "back to step email");
        assert_eq!(kind(to), ErrorKind::Other);
    }
}
