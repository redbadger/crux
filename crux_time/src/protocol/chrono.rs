use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for time operations
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Error)]
#[serde(rename_all = "camelCase")]
pub enum TimeError {
    #[error("invalid time")]
    InvalidTime,
    #[error("invalid Duration")]
    InvalidDuration,
    #[error("invalid Instant")]
    InvalidInstant,
}

impl TryFrom<TimeDelta> for crate::Duration {
    type Error = TimeError;

    fn try_from(value: TimeDelta) -> Result<Self, Self::Error> {
        let nanos = value.num_nanoseconds().ok_or(TimeError::InvalidDuration)? as u64;
        Ok(Self { nanos })
    }
}

impl TryFrom<crate::Duration> for TimeDelta {
    type Error = TimeError;

    fn try_from(value: crate::Duration) -> Result<Self, Self::Error> {
        let nanos = value
            .nanos
            .try_into()
            .map_err(|_| TimeError::InvalidDuration)?;
        Ok(TimeDelta::nanoseconds(nanos))
    }
}

impl TryFrom<crate::Instant> for DateTime<Utc> {
    type Error = TimeError;

    fn try_from(time: crate::Instant) -> Result<Self, Self::Error> {
        let seconds = i64::try_from(time.seconds).map_err(|_| TimeError::InvalidInstant)?;
        DateTime::<Utc>::from_timestamp(seconds, time.nanos).ok_or(TimeError::InvalidInstant)
    }
}

impl TryFrom<DateTime<Utc>> for crate::Instant {
    type Error = TimeError;

    fn try_from(time: DateTime<Utc>) -> Result<Self, Self::Error> {
        let seconds = time
            .timestamp()
            .try_into()
            .map_err(|_| TimeError::InvalidTime)?;
        let nanos = time.timestamp_subsec_nanos();
        Ok(crate::Instant { seconds, nanos })
    }
}

#[cfg(test)]
mod test {
    use chrono::{DateTime, TimeZone, Utc};

    #[test]
    fn duration_to_timedelta() {
        let duration = crate::Duration::new(1_000_000_000);
        let chrono_duration: chrono::TimeDelta = duration.try_into().unwrap();
        assert_eq!(chrono_duration.num_nanoseconds().unwrap(), 1_000_000_000);
    }

    #[test]
    fn timedelta_to_duration() {
        let chrono_duration = chrono::TimeDelta::nanoseconds(1_000_000_000);
        let duration: crate::Duration = chrono_duration.try_into().unwrap();
        assert_eq!(duration.nanos, 1_000_000_000);
    }

    #[test]
    fn instant_to_datetime_utc() {
        let instant = crate::Instant::new(1_000_000_000, 10);
        let chrono_time: DateTime<Utc> = instant.try_into().unwrap();
        assert_eq!(chrono_time.timestamp(), 1_000_000_000);
        assert_eq!(chrono_time.timestamp_subsec_nanos(), 10);
    }

    #[test]
    fn datetime_utc_to_instant() {
        let chrono_time: DateTime<Utc> = Utc.timestamp_opt(1_000_000_000, 10).unwrap();
        let instant: crate::Instant = chrono_time.try_into().unwrap();
        assert_eq!(instant.seconds, 1_000_000_000);
        assert_eq!(instant.nanos, 10);
    }
}
