use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Default, Clone, Eq)]
pub struct WeatherData {
    pub id: u64,
    pub main: String,
    pub description: String,
    pub icon: String,
}

impl fmt::Display for WeatherData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Weather: (id: {}, main: {}, description: {}, icon: {})",
            self.id, self.main, self.description, self.icon
        )
    }
}
