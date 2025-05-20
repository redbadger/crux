use js_sys::Date;
use shared::time::Instant;

pub fn get() -> Instant {
    let millis = Date::new_0().get_time();
    let seconds = millis / 1000f64;
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    Instant::new(seconds as u64, 0)
}
