//! The text and cursor of a one-line input, edited key by key.
use console::Key;

/// What a key did to a [`LineEditor`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edit {
    /// The text changed.
    Changed,
    /// Only the cursor moved.
    Moved,
    /// Nothing happened (the key is not an editing key, or there was nowhere to go).
    None,
}

/// The text of a one-line input and the cursor in it, counted in chars.
#[derive(Clone, Debug, Default)]
pub(crate) struct LineEditor {
    chars: Vec<char>,
    pos: usize,
}

// Some methods serve only optional features: history and completion replace the text,
// a password counts the chars around the cursor.
#[cfg_attr(
    not(all(feature = "password", feature = "history", feature = "completion")),
    allow(dead_code)
)]
impl LineEditor {
    /// An editor holding `initial`, with the cursor at the end.
    pub(crate) fn new(initial: &str) -> Self {
        let chars: Vec<char> = initial.chars().collect();
        LineEditor {
            pos: chars.len(),
            chars,
        }
    }

    /// The whole text.
    pub(crate) fn text(&self) -> String {
        self.chars.iter().collect()
    }

    /// The text before the cursor.
    pub(crate) fn before_cursor(&self) -> String {
        self.chars[..self.pos].iter().collect()
    }

    /// The text from the cursor on.
    pub(crate) fn after_cursor(&self) -> String {
        self.chars[self.pos..].iter().collect()
    }

    /// The number of chars before the cursor.
    pub(crate) fn cursor(&self) -> usize {
        self.pos
    }

    /// The number of chars.
    pub(crate) fn len(&self) -> usize {
        self.chars.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Whether the cursor is after the last char.
    pub(crate) fn at_end(&self) -> bool {
        self.pos == self.chars.len()
    }

    /// Replaces the text, with the cursor at the end. Returns whether the text changed.
    pub(crate) fn set(&mut self, text: &str) -> bool {
        let changed = !self.chars.iter().copied().eq(text.chars());
        self.wipe();
        self.chars.extend(text.chars());
        self.pos = self.chars.len();
        changed
    }

    /// Inserts `c` at the cursor and moves the cursor after it.
    pub(crate) fn insert(&mut self, c: char) -> bool {
        self.chars.insert(self.pos, c);
        self.pos += 1;
        true
    }

    /// Deletes the char before the cursor.
    pub(crate) fn backspace(&mut self) -> bool {
        if self.pos == 0 {
            return false;
        }
        self.pos -= 1;
        self.chars.remove(self.pos);
        true
    }

    /// Deletes the char under the cursor.
    pub(crate) fn delete(&mut self) -> bool {
        if self.pos == self.chars.len() {
            return false;
        }
        self.chars.remove(self.pos);
        true
    }

    /// Moves the cursor one char left.
    pub(crate) fn left(&mut self) -> bool {
        self.move_to(self.pos.saturating_sub(1))
    }

    /// Moves the cursor one char right.
    pub(crate) fn right(&mut self) -> bool {
        self.move_to((self.pos + 1).min(self.chars.len()))
    }

    /// Moves the cursor to the start.
    pub(crate) fn home(&mut self) -> bool {
        self.move_to(0)
    }

    /// Moves the cursor to the end.
    pub(crate) fn end(&mut self) -> bool {
        self.move_to(self.chars.len())
    }

    /// Moves the cursor to the start of the word before it: skips whitespace, then
    /// the word.
    pub(crate) fn word_left(&mut self) -> bool {
        let spaces = self.chars[..self.pos]
            .iter()
            .rev()
            .take_while(|c| c.is_whitespace())
            .count();
        let start = self.chars[..self.pos - spaces]
            .iter()
            .rposition(|c| c.is_whitespace())
            .map_or(0, |i| i + 1);
        self.move_to(start)
    }

    /// Moves the cursor past the next whitespace: to the end of the word under it,
    /// then over the whitespace after it, or to the end of the text.
    pub(crate) fn word_right(&mut self) -> bool {
        let rest = &self.chars[self.pos..];
        let target = match rest.iter().position(|c| c.is_whitespace()) {
            Some(space) => {
                let spaces = rest[space..]
                    .iter()
                    .take_while(|c| c.is_whitespace())
                    .count();
                self.pos + space + spaces
            }
            None => self.chars.len(),
        };
        self.move_to(target)
    }

    /// Applies an editing key.
    pub(crate) fn handle(&mut self, key: &Key) -> Edit {
        let (changed, moved) = match key {
            Key::Char(c) if !c.is_ascii_control() => (self.insert(*c), false),
            Key::Backspace => (self.backspace(), false),
            Key::Del => (self.delete(), false),
            Key::ArrowLeft => (false, self.left()),
            Key::ArrowRight => (false, self.right()),
            Key::Home => (false, self.home()),
            Key::End => (false, self.end()),
            Key::UnknownEscSeq(seq) if seq[..] == ['b'] => (false, self.word_left()),
            Key::UnknownEscSeq(seq) if seq[..] == ['f'] => (false, self.word_right()),
            _ => (false, false),
        };
        match (changed, moved) {
            (true, _) => Edit::Changed,
            (false, true) => Edit::Moved,
            (false, false) => Edit::None,
        }
    }

    /// Overwrites the text with zeros before dropping it (for passwords).
    #[cfg(feature = "password")]
    pub(crate) fn zeroize(&mut self) {
        zeroize::Zeroize::zeroize(&mut self.chars);
        self.pos = 0;
    }

    /// Empties the text, overwriting it first when it may be a secret.
    fn wipe(&mut self) {
        #[cfg(feature = "password")]
        self.zeroize();
        self.chars.clear();
    }

    fn move_to(&mut self, pos: usize) -> bool {
        let moved = pos != self.pos;
        self.pos = pos;
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str, pos: usize) -> LineEditor {
        let mut e = LineEditor::new(text);
        e.pos = pos;
        e
    }

    #[test]
    fn new_puts_the_cursor_at_the_end() {
        let e = LineEditor::new("añb");
        assert_eq!((e.cursor(), e.len(), e.text()), (3, 3, "añb".to_string()));
        assert!(e.at_end() && !e.is_empty());
        assert!(LineEditor::new("").is_empty());
    }

    #[test]
    fn text_splits_at_the_cursor_in_chars() {
        let e = at("日本語", 1);
        assert_eq!(e.before_cursor(), "日");
        assert_eq!(e.after_cursor(), "本語");
    }

    #[test]
    fn set_replaces_the_text_and_reports_a_change() {
        let mut e = at("abc", 1);
        assert!(!e.set("abc"));
        assert_eq!(e.cursor(), 3);
        assert!(e.set("ñu"));
        assert_eq!((e.text(), e.cursor()), ("ñu".to_string(), 2));
    }

    #[test]
    fn insert_and_delete_around_the_cursor() {
        let mut e = at("ac", 1);
        assert!(e.insert('ñ'));
        assert_eq!((e.text(), e.cursor()), ("añc".to_string(), 2));
        assert!(e.backspace());
        assert_eq!((e.text(), e.cursor()), ("ac".to_string(), 1));
        assert!(e.delete());
        assert_eq!((e.text(), e.cursor()), ("a".to_string(), 1));
        assert!(!e.delete());
        e.home();
        assert!(!e.backspace());
        assert_eq!(e.text(), "a");
    }

    #[test]
    fn moves_stop_at_the_ends() {
        let mut e = at("añ", 2);
        assert!(!e.right());
        assert!(e.left());
        assert_eq!(e.cursor(), 1);
        assert!(e.home());
        assert!(!e.left());
        assert!(!e.home());
        assert!(e.end());
        assert_eq!(e.cursor(), 2);
        assert!(!e.end());
    }

    #[test]
    fn word_left_goes_to_the_start_of_the_word() {
        let mut e = at("añb  cd ef", 10);
        assert!(e.word_left());
        assert_eq!(e.cursor(), 8); // "ef"
        assert!(e.word_left());
        assert_eq!(e.cursor(), 5); // "cd": skips the space first
        e.pos = 6; // inside "cd"
        e.word_left();
        assert_eq!(e.cursor(), 5);
        e.word_left();
        assert_eq!(e.cursor(), 0);
        assert!(!e.word_left());
    }

    #[test]
    fn word_right_goes_past_the_next_whitespace() {
        let mut e = at("añb  cd ef", 0);
        assert!(e.word_right());
        assert_eq!(e.cursor(), 5); // after "añb  "
        e.pos = 4; // on the second space
        e.word_right();
        assert_eq!(e.cursor(), 5);
        e.word_right();
        assert_eq!(e.cursor(), 8);
        e.word_right();
        assert_eq!(e.cursor(), 10);
        assert!(!e.word_right());
    }

    #[test]
    fn handle_maps_keys() {
        let mut e = LineEditor::new("a b");
        assert_eq!(e.handle(&Key::Char('ü')), Edit::Changed);
        assert_eq!(e.text(), "a bü");
        assert_eq!(e.handle(&Key::Char('\x07')), Edit::None);
        assert_eq!(e.handle(&Key::ArrowLeft), Edit::Moved);
        assert_eq!(e.handle(&Key::Del), Edit::Changed);
        assert_eq!(e.text(), "a b");
        assert_eq!(e.handle(&Key::Del), Edit::None);
        assert_eq!(e.handle(&Key::Backspace), Edit::Changed);
        assert_eq!(e.text(), "a ");
        assert_eq!(e.handle(&Key::Home), Edit::Moved);
        assert_eq!(e.handle(&Key::ArrowRight), Edit::Moved);
        assert_eq!(e.handle(&Key::End), Edit::Moved);
        assert_eq!(e.handle(&Key::UnknownEscSeq(vec!['b'])), Edit::Moved);
        assert_eq!(e.cursor(), 0);
        assert_eq!(e.handle(&Key::UnknownEscSeq(vec!['f'])), Edit::Moved);
        assert_eq!(e.cursor(), 2);
        assert_eq!(e.handle(&Key::Enter), Edit::None);
    }
}
