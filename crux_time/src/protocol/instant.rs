use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// The number of nanoseconds in seconds.
const NANOS_PER_SEC: u32 = 1_000_000_000;

/// Represents a point in time (UTC):
///
/// - seconds: number of seconds since the Unix epoch (1970-01-01T00:00:00Z)
/// - nanos: number of nanoseconds since the last second
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instant {
    pub(crate) seconds: u64,
    pub(crate) nanos: u32,
}

/// Create a new `Instant` from the given number of seconds and nanoseconds.
///
/// - seconds: number of seconds since the Unix epoch (1970-01-01T00:00:00Z)
/// - nanos: number of nanoseconds since the last second
///
/// Panics if the number of seconds
/// would overflow when converted to nanoseconds.
impl Instant {
    pub fn new(seconds: u64, nanos: u32) -> Self {
        if nanos >= NANOS_PER_SEC {
            panic!("nanos must be less than {}", NANOS_PER_SEC);
        }
        Self { seconds, nanos }
    }
}

impl From<SystemTime> for Instant {
    fn from(time: SystemTime) -> Self {
        let duration = time.duration_since(SystemTime::UNIX_EPOCH).unwrap();
        let seconds = duration.as_secs();
        let nanos = duration.subsec_nanos();
        Self { seconds, nanos }
    }
}

impl From<Instant> for SystemTime {
    fn from(time: Instant) -> Self {
        SystemTime::UNIX_EPOCH + std::time::Duration::new(time.seconds, time.nanos)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn new_instant() {
        let instant = Instant::new(1_000_000_000, 10);
        assert_eq!(instant.seconds, 1_000_000_000);
        assert_eq!(instant.nanos, 10);
    }

    #[test]
    #[should_panic]
    fn new_instant_invalid_nanos() {
        Instant::new(1_000_000_000, 1_000_000_000);
    }

    #[test]
    fn instant_to_std() {
        let actual: SystemTime = Instant::new(1_000_000_000, 10).into();
        let expected = SystemTime::UNIX_EPOCH + std::time::Duration::new(1_000_000_000, 10);
        assert_eq!(actual, expected);
    }

    #[test]
    fn std_to_instant() {
        let sys_time = SystemTime::UNIX_EPOCH + std::time::Duration::new(1_000_000_000, 10);
        let actual: Instant = sys_time.into();
        let expected = Instant::new(1_000_000_000, 10);
        assert_eq!(actual, expected);
    }
}
