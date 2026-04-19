//! Individual blocks of the current-weather response.
//!
//! Each sub-module is a direct mirror of one field group in the
//! [OpenWeatherMap response](https://openweathermap.org/current#fields_json).

mod clouds;
mod coord;
mod weather;
mod wind;

pub use clouds::*;
pub use coord::*;
pub use weather::WeatherData;
pub use wind::*;
