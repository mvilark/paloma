//! Provides validation for text inputs

/// Trait for input validators.
///
/// A generic implementation for `Fn(&str) -> Result<(), E>` is provided
/// to facilitate development.
pub trait InputValidator<T> {
    /// The error a failed validation returns, shown in the prompt's message row. The
    /// prompt's `validate` needs it to be [`IntoMessage`](crate::IntoMessage): any
    /// `Display` value (shown as an error) or a [`Message`](crate::Message) of any
    /// kind.
    type Err;

    /// Invoked with the value to validate.
    ///
    /// If this produces `Ok(())` then the value is used and parsed, if
    /// an error is returned validation fails with that error.
    fn validate(&mut self, input: &T) -> Result<(), Self::Err>;
}

impl<T, F, E> InputValidator<T> for F
where
    F: FnMut(&T) -> Result<(), E>,
{
    type Err = E;

    fn validate(&mut self, input: &T) -> Result<(), Self::Err> {
        self(input)
    }
}

/// Trait for password validators.
#[allow(clippy::ptr_arg)]
pub trait PasswordValidator {
    /// The error a failed validation returns, shown in the prompt's message row. The
    /// prompt's `validate` needs it to be [`IntoMessage`](crate::IntoMessage): any
    /// `Display` value (shown as an error) or a [`Message`](crate::Message) of any
    /// kind.
    type Err;

    /// Invoked with the value to validate.
    ///
    /// If this produces `Ok(())` then the value is used and parsed, if
    /// an error is returned validation fails with that error.
    fn validate(&self, input: &String) -> Result<(), Self::Err>;
}

impl<F, E> PasswordValidator for F
where
    F: Fn(&String) -> Result<(), E>,
{
    type Err = E;

    fn validate(&self, input: &String) -> Result<(), Self::Err> {
        self(input)
    }
}
