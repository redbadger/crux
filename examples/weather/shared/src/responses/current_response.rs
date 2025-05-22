use crate::responses::response_elements::Clouds;
use crate::responses::response_elements::Coord;
use crate::responses::response_elements::Weather;
use crate::responses::response_elements::Wind;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Default, Clone)]
pub struct Sys {
    #[serde(rename = "type")]
    pub sys_type: usize,
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
            self.sys_type, self.id, self.country, self.sunrise, self.sunset,
        )
    }
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Default, Copy, Clone)]
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
            self.temp,
            self.feels_like,
            self.temp_min,
            self.temp_max,
            self.pressure,
            self.humidity,

        )
    }
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Default, Clone)]
pub struct CurrentResponse {
    pub coord: Coord,
    pub weather: Vec<Weather>,
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

impl fmt::Display for CurrentResponse {
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

// Test helpers

pub const SAMPLE_CURRENT_RESPONSE_JSON: &str = r#"{
    "main": {
        "temp": 20.0,
        "feels_like": 18.0,
        "temp_min": 18.0,
        "temp_max": 22.0,
        "pressure": 1013,
        "humidity": 50
    },
    "coord": {
        "lat": 33.456789,
        "lon": -112.037222
    },
    "weather": [{
        "id": 800,
        "main": "Clear",
        "description": "clear sky",
        "icon": "01d"
    }],
    "base": "",
    "visibility": 10000,
    "wind": {
        "speed": 4.1,
        "deg": 280,
        "gust": 5.2
    },
    "clouds": {
        "all": 0
    },
    "dt": 1716216000,
    "sys": {
        "id": 1,
        "country": "US",
        "type": 1,
        "sunrise": 1716216000,
        "sunset": 1716216000
    },
    "timezone": 1,
    "id": 1,
    "name": "Phoenix",
    "cod": 200
}"#;

pub static SAMPLE_CURRENT_RESPONSE: Lazy<CurrentResponse> =
    Lazy::new(|| serde_json::from_str(SAMPLE_CURRENT_RESPONSE_JSON).unwrap());
