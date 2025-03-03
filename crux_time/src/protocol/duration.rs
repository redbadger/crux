use serde::{Deserialize, Serialize};

/// The number of nanoseconds in seconds.
const NANOS_PER_SEC: u32 = 1_000_000_000;
/// The number of nanoseconds in a millisecond.
const NANOS_PER_MILLI: u32 = 1_000_000;

/// Represents a duration of time, internally stored as nanoseconds
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Duration {
    pub(crate) nanos: u64,
}

impl Duration {
    /// Create a new `Duration` from the given number of nanoseconds.
    pub fn new(nanos: u64) -> Self {
        Self { nanos }
    }

    /// Create a new `Duration` from the given number of milliseconds.
    ///
    /// Panics if the number of milliseconds
    /// would overflow when converted to nanoseconds.
    pub fn from_millis(millis: u64) -> Self {
        let nanos = millis
            .checked_mul(NANOS_PER_MILLI as u64)
            .expect("millis overflow");
        Self { nanos }
    }

    /// Create a new `Duration` from the given number of seconds.
    ///
    /// Panics if the number of seconds
    /// would overflow when converted to nanoseconds.
    pub fn from_secs(seconds: u64) -> Self {
        let nanos = seconds
            .checked_mul(NANOS_PER_SEC as u64)
            .expect("seconds overflow");
        Self { nanos }
    }
}

impl From<std::time::Duration> for Duration {
    fn from(duration: std::time::Duration) -> Self {
        Duration {
            nanos: duration.as_nanos() as u64,
        }
    }
}

impl From<Duration> for std::time::Duration {
    fn from(duration: Duration) -> Self {
        std::time::Duration::from_nanos(duration.nanos)
    }
}

#[cfg(test)]
mod test {
    use super::Duration;
    use std::time::Duration as StdDuration;

    #[test]
    fn duration_from_millis() {
        let duration = Duration::from_millis(1_000);
        assert_eq!(duration.nanos, 1_000_000_000);
    }

    #[test]
    fn duration_from_secs() {
        let duration = Duration::from_secs(1);
        assert_eq!(duration.nanos, 1_000_000_000);
    }

    #[test]
    fn std_into_duration() {
        let actual: Duration = StdDuration::from_millis(100).into();
        let expected = Duration { nanos: 100_000_000 };
        assert_eq!(actual, expected);
    }

    #[test]
    fn duration_into_std() {
        let actual: StdDuration = Duration { nanos: 100_000_000 }.into();
        let expected = StdDuration::from_nanos(100_000_000);
        assert_eq!(actual, expected);
    }
}
