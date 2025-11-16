use crate::weather::model::response_elements::Clouds;
use crate::weather::model::response_elements::Coord;
use crate::weather::model::response_elements::WeatherData;
use crate::weather::model::response_elements::Wind;
use derive_builder::Builder;

use facet::Facet;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const WEATHER_URL: &str = "https://api.openweathermap.org/data/2.5/weather";

#[derive(
    Facet, Debug, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Default, Clone,
)]
pub struct Sys {
    #[serde(rename = "type")]
    pub type_: usize,
    pub id: usize,
    pub country: String,
    pub sunrise: usize,
    pub sunset: usize,
}

impl fmt::Display for Sys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "System: (type: {}, id: {}, country: {}, sunrise: {}, sunset: {})",
            self.type_, self.id, self.country, self.sunrise, self.sunset,
        )
    }
}

#[derive(Facet, Debug, Serialize, Deserialize, PartialOrd, PartialEq, Default, Copy, Clone)]
pub struct Main {
    pub temp: f64,
    pub feels_like: f64,
    pub temp_min: f64,
    pub temp_max: f64,
    pub pressure: u64,
    pub humidity: u64,
}

impl fmt::Display for Main {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Main: (temp: {}, feels_like: {}, temp_min: {}, temp_max: {}, pressure: {}, humidity: {})",
            self.temp, self.feels_like, self.temp_min, self.temp_max, self.pressure, self.humidity,
        )
    }
}

#[derive(Facet, Debug, Serialize, Deserialize, PartialOrd, PartialEq, Default, Clone, Builder)]
#[builder(setter(into))]
pub struct CurrentWeatherResponse {
    pub coord: Coord,
    pub weather: Vec<WeatherData>,
    pub base: String,
    pub main: Main,
    pub visibility: usize,
    pub wind: Wind,
    pub clouds: Clouds,
    pub dt: usize,
    pub sys: Sys,
    pub timezone: i64,
    pub id: usize,
    pub name: String,
    pub cod: usize,
}

impl fmt::Display for CurrentWeatherResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CurrentResponse: (Coord: {:?}, weather: {:?}, base: {:?}, main: {:?}, visibility: {:?}, wind: {:?}, clouds: {:?}, dt: {:?}, sys: {:?}, timezone: {:?}, id: {:?}, name: {:?}: cod: {:?})",
            self.coord,
            self.weather,
            self.base,
            self.main,
            self.visibility,
            self.wind,
            self.clouds,
            self.dt,
            self.sys,
            self.timezone,
            self.id,
            self.name,
            self.cod,
        )
    }
}
