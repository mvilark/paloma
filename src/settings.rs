//! The one mechanism behind option groups and component tokens.
//!
//! A settings group is a struct of optional fields: a field left unset falls through to
//! the next level (a status group to its family, a family to `general`, then the
//! built-in default). [`settings!`] writes the struct, a documented setter per field and
//! a `merged` method that fills unset fields from a fallback, so options and tokens cascade the same way.

/// Declares a settings group.
///
/// ```ignore
/// settings! {
///     /// Docs for the group.
///     pub struct MessageOptions {
///         /// Docs for the setter.
///         stream: Stream,
///     }
/// }
/// ```
///
/// Every field becomes `pub(crate) field: Option<T>` with a public setter
/// `fn field(self, val: T) -> Self`. `merged(&fallback)` fills unset fields from
/// `fallback`.
macro_rules! settings {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $(#[$fmeta:meta])*
                $field:ident: $ty:ty,
            )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Default, Debug)]
        pub struct $name {
            $(pub(crate) $field: Option<$ty>,)*
        }

        impl $name {
            $(
                $(#[$fmeta])*
                pub fn $field(mut self, val: $ty) -> Self {
                    self.$field = Some(val);
                    self
                }
            )*

            /// Fills every unset field from `fallback`.
            #[allow(dead_code)]
            pub(crate) fn merged(&self, fallback: &Self) -> Self {
                Self {
                    $($field: self.$field.clone().or_else(|| fallback.$field.clone()),)*
                }
            }
        }
    };
}
pub(crate) use settings;
