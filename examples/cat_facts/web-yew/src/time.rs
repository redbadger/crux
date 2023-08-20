use anyhow::Result;
use js_sys::Date;

pub fn get() -> Result<String> {
    let date = Date::new_0();

    Ok(format!("{}", date.to_iso_string()))
}
