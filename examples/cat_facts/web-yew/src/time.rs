use js_sys::Date;

pub fn get() -> u32 {
    Date::new_0().get_utc_milliseconds()
}
