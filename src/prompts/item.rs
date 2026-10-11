//! Items of the list prompts: [`Item`], what they can be made from ([`IntoItem`]) and
//! the navigation that skips disabled ones.
use std::{fmt, io};

use crate::Result;

/// An item of a list prompt ([`Select`](crate::Select),
/// [`MultiSelect`](crate::MultiSelect), `FuzzySelect`), with an optional description
/// and a disabled state.
///
/// Plain text works as an item too (see [`IntoItem`]), so `.items(["a", "b"])` needs no
/// `Item`:
///
/// ```rust,no_run
/// use paloma::{Item, Select};
///
/// let i = Select::new()
///     .prompt("Plan")
///     .item(Item::new("Free").description("1 device"))
///     .item(Item::new("Team").description("10 devices"))
///     .item(Item::new("Enterprise").description("talk to sales").disabled(true))
///     .interact()?;
/// # Ok::<(), paloma::Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct Item {
    text: String,
    description: Option<String>,
    disabled: bool,
}

// `Item` must never implement `Display`: the blanket `impl<T: Display> IntoItem for T`
// would then overlap with `impl IntoItem for Item`.

impl Item {
    /// An item that shows `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Item {
            text: text.into(),
            description: None,
            disabled: false,
        }
    }

    /// Extra text drawn after the item (see the `item_description_style` and
    /// `item_description_gap` prompt tokens).
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// A disabled item is shown but cannot be focused, chosen or checked.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The text of the item: what a prompt matches on and what the answered line shows.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// The description, if there is one.
    pub(crate) fn description_text(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Whether the item is disabled.
    pub(crate) fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// Something a list prompt can show as an item: an [`Item`], or any `Display` value as
/// its text.
pub trait IntoItem {
    /// The item.
    fn into_item(self) -> Item;
}

impl IntoItem for Item {
    fn into_item(self) -> Item {
        self
    }
}

impl<T: fmt::Display> IntoItem for T {
    fn into_item(self) -> Item {
        Item::new(self.to_string())
    }
}

/// The first index at or after `from` (or at or before it when `forward` is false) for
/// which `enabled` is true, wrapping around the `len` items. `None` when no index is
/// enabled.
pub(crate) fn next_where(
    len: usize,
    from: usize,
    forward: bool,
    enabled: impl Fn(usize) -> bool,
) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let from = from.min(len - 1);
    (0..len)
        .map(|step| {
            if forward {
                (from + step) % len
            } else {
                (from + len - step) % len
            }
        })
        .find(|&i| enabled(i))
}

/// The first enabled item at or after `from` (at or before it when `forward` is
/// false), wrapping around. `None` when every item is disabled.
pub(crate) fn next_enabled(items: &[Item], from: usize, forward: bool) -> Option<usize> {
    next_where(items.len(), from, forward, |i| !items[i].disabled)
}

/// The item after (`forward`) or before the highlighted one `from`, skipping disabled
/// items and wrapping around. Without a highlighted item (`from` is `!0`) the first
/// (or last) enabled item. `None` when no item can be reached.
pub(crate) fn step_enabled(items: &[Item], from: usize, forward: bool) -> Option<usize> {
    let len = items.len();
    if len == 0 {
        None
    } else if from >= len {
        next_enabled(items, if forward { 0 } else { len - 1 }, forward)
    } else if forward {
        next_enabled(items, (from + 1) % len, true)
    } else {
        next_enabled(items, (from + len - 1) % len, false)
    }
}

/// `index` if it is an index into `len` items, an error otherwise (a preset or default
/// that names an item that does not exist).
pub(crate) fn check_index(index: usize, len: usize) -> Result<usize> {
    if index < len {
        Ok(index)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("item {index} does not exist ({len} items)"),
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(disabled: &[bool]) -> Vec<Item> {
        disabled
            .iter()
            .enumerate()
            .map(|(i, &d)| Item::new(format!("item {i}")).disabled(d))
            .collect()
    }

    #[test]
    fn plain_values_and_items_become_items() {
        let plain = "a".into_item();
        assert_eq!(plain.text(), "a");
        assert_eq!(plain.description_text(), None);
        assert!(!plain.is_disabled());
        let plain = 42.into_item();
        assert_eq!(plain.text(), "42");
        let rich = Item::new("b")
            .description("more")
            .disabled(true)
            .into_item();
        assert_eq!(rich.text(), "b");
        assert_eq!(rich.description_text(), Some("more"));
        assert!(rich.is_disabled());
    }

    #[test]
    fn next_enabled_skips_disabled_items_and_wraps() {
        let list = items(&[false, true, true, false, true]);
        // Forward: at or after `from`.
        assert_eq!(next_enabled(&list, 0, true), Some(0));
        assert_eq!(next_enabled(&list, 1, true), Some(3));
        assert_eq!(next_enabled(&list, 4, true), Some(0));
        // Backward: at or before `from`.
        assert_eq!(next_enabled(&list, 3, false), Some(3));
        assert_eq!(next_enabled(&list, 2, false), Some(0));
        assert_eq!(next_enabled(&list, 4, false), Some(3));
        // Wrapping backwards from the start.
        let list = items(&[true, false, true]);
        assert_eq!(next_enabled(&list, 0, false), Some(1));
        assert_eq!(next_enabled(&list, 2, true), Some(1));
    }

    #[test]
    fn next_enabled_is_none_when_every_item_is_disabled_or_there_are_none() {
        assert_eq!(next_enabled(&items(&[true, true]), 0, true), None);
        assert_eq!(next_enabled(&items(&[true, true]), 1, false), None);
        assert_eq!(next_enabled(&[], 0, true), None);
        // An out of range `from` is clamped.
        assert_eq!(next_enabled(&items(&[false, false]), 9, false), Some(1));
    }

    #[test]
    fn step_enabled_moves_one_enabled_item_at_a_time() {
        let list = items(&[false, true, false, true]);
        assert_eq!(step_enabled(&list, 0, true), Some(2));
        assert_eq!(step_enabled(&list, 2, true), Some(0));
        assert_eq!(step_enabled(&list, 0, false), Some(2));
        assert_eq!(step_enabled(&list, 2, false), Some(0));
        // Nothing highlighted yet: the first or the last enabled item.
        assert_eq!(step_enabled(&list, !0, true), Some(0));
        assert_eq!(step_enabled(&list, !0, false), Some(2));
        let list = items(&[true, false, true]);
        assert_eq!(step_enabled(&list, !0, true), Some(1));
        assert_eq!(step_enabled(&list, !0, false), Some(1));
        // With a single enabled item the highlight stays on it.
        assert_eq!(step_enabled(&list, 1, true), Some(1));
        assert_eq!(step_enabled(&items(&[true]), !0, true), None);
    }

    #[test]
    fn check_index_rejects_items_that_do_not_exist() {
        assert!(check_index(1, 2).is_ok());
        assert!(check_index(2, 2).is_err());
    }
}
