use serde::{Deserialize, Serialize};

use crate::{error::TimeResult, TimeError};

/// The number of nanoseconds in seconds.
pub(crate) const NANOS_PER_SEC: u32 = 1_000_000_000;
/// The number of nanoseconds in a millisecond.
const NANOS_PER_MILLI: u32 = 1_000_000;

/// Represents a duration of time, internally stored as nanoseconds
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Duration {
    nanos: u64,
}

impl Duration {
    /// Create a new `Duration` from the given number of nanoseconds.
    pub fn new(nanos: u64) -> Self {
        Self { nanos }
    }

    /// Create a new `Duration` from the given number of milliseconds.
    ///
    /// Errors with [`TimeError::InvalidDuration`] if the number of milliseconds
    /// would overflow when converted to nanoseconds.
    pub fn from_millis(millis: u64) -> TimeResult<Self> {
        let nanos = millis
            .checked_mul(NANOS_PER_MILLI as u64)
            .ok_or(TimeError::InvalidDuration)?;
        Ok(Self { nanos })
    }

    /// Create a new `Duration` from the given number of seconds.
    ///
    /// Errors with [`TimeError::InvalidDuration`] if the number of seconds
    /// would overflow when converted to nanoseconds.
    pub fn from_secs(seconds: u64) -> TimeResult<Self> {
        let nanos = seconds
            .checked_mul(NANOS_PER_SEC as u64)
            .ok_or(TimeError::InvalidDuration)?;
        Ok(Self { nanos })
    }
}

#[cfg(feature = "chrono")]
impl TryFrom<chrono::TimeDelta> for Duration {
    type Error = TimeError;

    fn try_from(value: chrono::TimeDelta) -> Result<Self, Self::Error> {
        let nanos = value.num_nanoseconds().ok_or(TimeError::InvalidDuration)? as u64;
        Ok(Self { nanos })
    }
}

#[cfg(feature = "chrono")]
impl TryFrom<Duration> for chrono::TimeDelta {
    type Error = TimeError;

    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        let nanos = value
            .nanos
            .try_into()
            .map_err(|_| TimeError::InvalidDuration)?;
        Ok(chrono::TimeDelta::nanoseconds(nanos))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn duration_from_millis() {
        let duration = Duration::from_millis(1_000).unwrap();
        assert_eq!(duration.nanos, 1_000_000_000);
    }

    #[test]
    fn duration_from_secs() {
        let duration = Duration::from_secs(1).unwrap();
        assert_eq!(duration.nanos, 1_000_000_000);
    }
}

#[cfg(feature = "chrono")]
#[cfg(test)]
mod chrono_test {
    use super::*;

    #[test]
    fn duration_to_timedelta() {
        let duration = Duration::new(1_000_000_000);
        let chrono_duration: chrono::TimeDelta = duration.try_into().unwrap();
        assert_eq!(chrono_duration.num_nanoseconds().unwrap(), 1_000_000_000);
    }

    #[test]
    fn timedelta_to_duration() {
        let chrono_duration = chrono::TimeDelta::nanoseconds(1_000_000_000);
        let duration: Duration = chrono_duration.try_into().unwrap();
        assert_eq!(duration.nanos, 1_000_000_000);
    }
}
