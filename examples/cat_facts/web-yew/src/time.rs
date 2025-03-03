use js_sys::Date;
use shared::time::Instant;

pub fn get() -> Instant {
    let millis = Date::new_0().get_time();
    let seconds = millis / 1000f64;
    let nanos = (millis % 1000f64) as u32 * 1_000_000;
    Instant::new(seconds as u64, nanos)
}
