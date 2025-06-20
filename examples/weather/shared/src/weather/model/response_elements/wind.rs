use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Default, Copy, Clone)]
pub struct Wind {
    pub speed: f64,
    pub deg: i64,
    pub gust: Option<f64>,
}

impl fmt::Display for Wind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.gust {
            Some(gust) => write!(
                f,
                "Wind: (speed: {}, degree: {}, gust: {})",
                self.speed, self.deg, gust
            ),
            None => write!(f, "Wind: (speed: {}, degree: {})", self.speed, self.deg,),
        }
    }
}
