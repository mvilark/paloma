use std::{fmt, io::Error as IoError, result::Result as StdResult};

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
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IO(io) => write!(f, "IO error: {}", io),
            Self::Cancelled => write!(f, "cancelled"),
            Self::NoDefault => write!(f, "no default for an unattended prompt"),
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
    /// `Cancelled` becomes `ErrorKind::Interrupted` and `NoDefault`
    /// `ErrorKind::InvalidInput`.
    fn from(value: Error) -> Self {
        match value {
            Error::IO(err) => err,
            Error::Cancelled => IoError::new(std::io::ErrorKind::Interrupted, value),
            Error::NoDefault => IoError::new(std::io::ErrorKind::InvalidInput, value),
        }
    }
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
    }
}
