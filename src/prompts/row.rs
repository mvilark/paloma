//! The message row under a prompt: its description, its `on_change` closure and what
//! it shows right now.
use std::sync::{Arc, Mutex};

use crate::{output::message::Message, theme::MessageSource};

/// An `on_change` closure: the message for the prompt's current value, or `None` for
/// the description.
pub(crate) type OnChange<'a, V> = Arc<Mutex<dyn FnMut(&V) -> Option<Message<'static>> + Send + 'a>>;

/// What the row shows, apart from the description.
#[derive(Clone)]
enum RowState {
    /// The description, if there is one.
    Description,
    /// What `on_change` returned for the current value.
    Change(Message<'static>),
    /// A validation error, until the value changes.
    Invalid(Message<'static>),
}

/// The row under a prompt whose value is a `V`. The description lives in the prompt's
/// [`PromptBase`](crate::prompts::base::PromptBase), shared by every prompt.
pub(crate) struct Row<'a, V: ?Sized> {
    on_change: Option<OnChange<'a, V>>,
    state: RowState,
}

impl<V: ?Sized> Clone for Row<'_, V> {
    fn clone(&self) -> Self {
        Row {
            on_change: self.on_change.clone(),
            state: self.state.clone(),
        }
    }
}

impl<V: ?Sized> Default for Row<'_, V> {
    fn default() -> Self {
        Row {
            on_change: None,
            state: RowState::Description,
        }
    }
}

impl<'a, V: ?Sized> Row<'a, V> {
    /// Sets the `on_change` closure, replacing an earlier one.
    pub(crate) fn on_change(&mut self, f: impl FnMut(&V) -> Option<Message<'static>> + Send + 'a) {
        self.on_change = Some(Arc::new(Mutex::new(f)));
    }

    /// The value changed: asks `on_change` what to show, or goes back to the
    /// description. A validation error goes away.
    pub(crate) fn changed(&mut self, value: &V) {
        let message = match &self.on_change {
            Some(f) => f.lock().unwrap_or_else(|e| e.into_inner())(value),
            None => None,
        };
        self.state = match message {
            Some(message) => RowState::Change(message),
            None => RowState::Description,
        };
    }

    /// A validation failed: `message` shows until the value changes.
    pub(crate) fn invalid(&mut self, message: Message<'static>) {
        self.state = RowState::Invalid(message);
    }

    /// The row shows a validation error.
    pub(crate) fn is_invalid(&self) -> bool {
        matches!(self.state, RowState::Invalid(_))
    }

    /// What the row shows and where it comes from, or `None` when it is empty.
    pub(crate) fn current<'d>(
        &'d self,
        description: Option<&'d Message<'static>>,
    ) -> Option<(&'d Message<'static>, MessageSource)> {
        match &self.state {
            RowState::Description => description.map(|d| (d, MessageSource::Description)),
            RowState::Change(message) => Some((message, MessageSource::Change)),
            RowState::Invalid(message) => Some((message, MessageSource::Validation)),
        }
    }
}

/// A validator: `Some(error)` when the value is refused.
type Check<'a, V> = Arc<Mutex<dyn FnMut(&V) -> Option<Message<'static>> + 'a>>;

/// The validators of a prompt, run in the order they were added; the first error wins.
pub(crate) struct Validators<'a, V: ?Sized>(Vec<Check<'a, V>>);

impl<V: ?Sized> Clone for Validators<'_, V> {
    fn clone(&self) -> Self {
        Validators(self.0.clone())
    }
}

impl<V: ?Sized> Default for Validators<'_, V> {
    fn default() -> Self {
        Validators(Vec::new())
    }
}

impl<'a, V: ?Sized> Validators<'a, V> {
    /// Adds a validator after the others.
    pub(crate) fn push(&mut self, f: impl FnMut(&V) -> Option<Message<'static>> + 'a) {
        self.0.push(Arc::new(Mutex::new(f)));
    }

    /// The first error any validator returns for `value`.
    pub(crate) fn check(&self, value: &V) -> Option<Message<'static>> {
        self.0
            .iter()
            .find_map(|f| f.lock().unwrap_or_else(|e| e.into_inner())(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Status;

    fn shown(
        row: &Row<str>,
        description: Option<&Message<'static>>,
    ) -> Option<(String, MessageSource)> {
        row.current(description)
            .map(|(m, source)| (m.text().to_string(), source))
    }

    #[test]
    fn the_row_shows_the_latest_source() {
        let description = Message::description("3 to 16 letters");
        let mut row: Row<str> = Row::default();
        row.on_change(|text: &str| (text.len() < 3).then(|| Message::error("Too short")));

        // Description first.
        let d = Some(&description);
        assert_eq!(
            shown(&row, d),
            Some(("3 to 16 letters".into(), MessageSource::Description))
        );
        // A change replaces it, `None` brings it back.
        row.changed("ab");
        assert_eq!(
            shown(&row, d),
            Some(("Too short".into(), MessageSource::Change))
        );
        row.changed("abc");
        assert_eq!(
            shown(&row, d),
            Some(("3 to 16 letters".into(), MessageSource::Description))
        );
        // An error replaces it until the next change.
        row.invalid(Message::error("Taken"));
        assert!(row.is_invalid());
        assert_eq!(
            shown(&row, d),
            Some(("Taken".into(), MessageSource::Validation))
        );
        row.changed("abcd");
        assert!(!row.is_invalid());
        assert_eq!(
            shown(&row, d),
            Some(("3 to 16 letters".into(), MessageSource::Description))
        );
    }

    #[test]
    fn without_on_change_or_description_the_row_is_empty() {
        let mut row: Row<[usize]> = Row::default();
        assert!(row.current(None).is_none());
        row.changed(&[1, 2]);
        assert!(row.current(None).is_none());
        row.invalid(Message::error("Pick one"));
        let (message, source) = row.current(None).unwrap();
        assert_eq!(
            (message.status(), source),
            (Status::Error, MessageSource::Validation)
        );
    }

    #[test]
    fn the_last_on_change_wins_and_clones_share_it() {
        let mut row: Row<usize> = Row::default();
        row.on_change(|_| Some(Message::hint("first")));
        row.on_change(|i| Some(Message::hint(format!("item {i}"))));
        let mut copy = row.clone();
        copy.changed(&2);
        assert_eq!(copy.current(None).unwrap().0.text(), "item 2");
        assert!(row.current(None).is_none());
    }

    #[test]
    fn validators_run_in_order_and_the_first_error_wins() {
        let mut v: Validators<usize> = Validators::default();
        v.push(|&i| (i == 0).then(|| Message::error("zero")));
        v.push(|&i| (i < 2).then(|| Message::error("small")));
        assert_eq!(v.check(&0).unwrap().text(), "zero");
        assert_eq!(v.check(&1).unwrap().text(), "small");
        assert!(v.check(&2).is_none());
    }
}
