/// Trait for completion handling.
pub trait Completion {
    /// Returns the completed text for `input`, or `None` when there is nothing to complete.
    ///
    /// Called when the user presses `Tab` or `→`; the result replaces the whole input.
    fn get(&self, input: &str) -> Option<String>;
}
