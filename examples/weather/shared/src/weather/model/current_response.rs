use crate::weather::model::response_elements::Clouds;
use crate::weather::model::response_elements::Coord;
use crate::weather::model::response_elements::WeatherData;
use crate::weather::model::response_elements::Wind;
use derive_builder::Builder;
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

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Default, Clone, Builder)]
#[builder(setter(into))]
pub struct CurrentResponse {
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

pub static SAMPLE_CURRENT_RESPONSE: Lazy<CurrentResponse> = Lazy::new(|| {
    CurrentResponseBuilder::default()
        .main(Main {
            temp: 20.0,
            feels_like: 18.0,
            temp_min: 18.0,
            temp_max: 22.0,
            pressure: 1013,
            humidity: 50,
        })
        .coord(Coord {
            lat: 33.456_789,
            lon: -112.037_222,
        })
        .weather(vec![WeatherData {
            id: 800,
            main: "Clear".to_string(),
            description: "clear sky".to_string(),
            icon: "01d".to_string(),
        }])
        .base(String::new())
        .visibility(10000_usize)
        .wind(Wind {
            speed: 4.1,
            deg: 280,
            gust: Some(5.2),
        })
        .clouds(Clouds { all: 0 })
        .dt(1_716_216_000_usize)
        .sys(Sys {
            id: 1,
            country: "US".to_string(),
            sys_type: 1,
            sunrise: 1_716_216_000,
            sunset: 1_716_216_000,
        })
        .timezone(1)
        .id(1_usize)
        .name("Phoenix".to_string())
        .cod(200_usize)
        .build()
        .expect("Failed to build sample response")
});

pub static SAMPLE_CURRENT_RESPONSE_JSON: Lazy<String> =
    Lazy::new(|| serde_json::to_string(&*SAMPLE_CURRENT_RESPONSE).unwrap());
