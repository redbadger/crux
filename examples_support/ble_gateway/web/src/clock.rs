//! When each log line was logged, as wall-clock time.
//!
//! The core numbers its log lines but doesn't know the time: reading the clock is a side
//! effect, and asking the shell for it (`crux_time`) on every line would be a round trip per
//! line for something only this view needs. The shell already knows when each line
//! appeared, so it stamps them.

use std::{cell::RefCell, collections::BTreeMap};

use gateway_core::LogLine;

thread_local! {
    /// `seq` → time, for the lines the log still holds.
    static STAMPS: RefCell<BTreeMap<u64, String>> = const { RefCell::new(BTreeMap::new()) };
}

/// Stamp the lines not seen before with the time now, and forget those the log dropped.
pub fn stamp(log: &[LogLine]) {
    STAMPS.with_borrow_mut(|stamps| {
        let mut now = None;
        for line in log {
            stamps
                .entry(line.seq)
                .or_insert_with(|| now.get_or_insert_with(now_hms).clone());
        }
        if let Some(oldest) = log.iter().map(|line| line.seq).min() {
            stamps.retain(|seq, _| *seq >= oldest);
        }
    });
}

/// When the line numbered `seq` was logged, as `HH:MM:SS.s`.
pub fn time_of(seq: u64) -> String {
    STAMPS.with_borrow(|stamps| stamps.get(&seq).cloned().unwrap_or_default())
}

fn now_hms() -> String {
    let now = js_sys::Date::new_0();
    format!(
        "{:02}:{:02}:{:02}.{}",
        now.get_hours(),
        now.get_minutes(),
        now.get_seconds(),
        now.get_milliseconds() / 100
    )
}
