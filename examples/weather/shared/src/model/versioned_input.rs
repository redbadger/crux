// ANCHOR: versioned_input
/// A text input that tracks a version number, incremented on each update.
///
/// Used to correlate async responses (e.g. search results) with the input
/// that triggered them, so stale responses can be discarded. Capture the
/// version when an effect is started, then check it against the current
/// version via [`Self::is_current`] when the response arrives.
#[derive(Debug, Default)]
pub struct VersionedInput {
    version: usize,
    value: String,
}

impl VersionedInput {
    /// Updates the input value and bumps the version, returning the new
    /// version.
    pub fn update(&mut self, value: String) -> usize {
        self.version = self.version.wrapping_add(1);
        self.value = value;
        self.version
    }

    /// Returns the current input text.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the current version number.
    pub fn version(&self) -> usize {
        self.version
    }

    /// Whether the given version matches the current one — used to discard
    /// responses from stale inputs.
    pub fn is_current(&self, version: usize) -> bool {
        self.version == version
    }
}
// ANCHOR_END: versioned_input

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_zero() {
        let input = VersionedInput::default();
        assert_eq!(input.value(), "");
        assert!(input.is_current(0));
    }

    #[test]
    fn update_bumps_version() {
        let mut input = VersionedInput::default();
        let v1 = input.update("a".to_string());
        assert_eq!(v1, 1);
        assert_eq!(input.value(), "a");
        assert!(input.is_current(1));
        assert!(!input.is_current(0));

        let v2 = input.update("ab".to_string());
        assert_eq!(v2, 2);
        assert!(input.is_current(2));
        assert!(!input.is_current(1));
    }
}
