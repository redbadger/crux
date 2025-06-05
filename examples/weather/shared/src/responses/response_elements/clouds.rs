use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Debug, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Default, Copy, Clone,
)]
pub struct Clouds {
    pub all: i64,
}

impl fmt::Display for Clouds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Clouds: (all: {})", self.all,)
    }
}
