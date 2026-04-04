/// A text input that tracks a version number, incremented on each update.
///
/// Used to correlate async responses (e.g. search results) with the input
/// that triggered them, so stale responses can be discarded.
#[derive(Debug, Default)]
pub struct VersionedInput {
    version: usize,
    value: String,
}

impl VersionedInput {
    /// Update the input value and bump the version. Returns the new version.
    pub fn update(&mut self, value: String) -> usize {
        self.version = self.version.wrapping_add(1);
        self.value = value;
        self.version
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn version(&self) -> usize {
        self.version
    }

    pub fn is_current(&self, version: usize) -> bool {
        self.version == version
    }
}

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
