use std::fmt;

pub fn display_option<T: fmt::Display>(option_string: &Option<T>) -> String {
    match option_string {
        Some(string) => string.to_string(),
        None => "None".to_string(),
    }
}
