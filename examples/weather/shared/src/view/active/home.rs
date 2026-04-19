use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    effects::{http::weather::model::current_response::CurrentWeatherResponse, location::Location},
    model::active::home::{FavoriteWeather, FavoriteWeatherState, HomeScreen, LocalWeather},
};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct HomeViewModel {
    pub local_weather: LocalWeatherViewModel,
    pub favorites: Vec<FavoriteWeatherViewModel>,
}

/// The [`Default`] variant is `CheckingPermission` — the state before any
/// work has been done. Used as the fallback when the web shell projects the
/// top-level view model into a `Memo<HomeViewModel>`.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[repr(C)]
pub enum LocalWeatherViewModel {
    #[default]
    CheckingPermission,
    LocationDisabled,
    FetchingLocation,
    FetchingWeather,
    Fetched(Box<CurrentWeatherResponse>),
    Failed,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FavoriteWeatherViewModel {
    pub name: String,
    pub location: Location,
    pub weather: FavoriteWeatherStateViewModel,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum FavoriteWeatherStateViewModel {
    Fetching,
    Fetched(Box<CurrentWeatherResponse>),
    Failed,
}

impl From<&HomeScreen> for HomeViewModel {
    fn from(home: &HomeScreen) -> Self {
        HomeViewModel {
            local_weather: (&home.current_weather).into(),
            favorites: home.favorites_weather.iter().map(From::from).collect(),
        }
    }
}

impl From<&LocalWeather> for LocalWeatherViewModel {
    fn from(lw: &LocalWeather) -> Self {
        match lw {
            LocalWeather::CheckingPermission => LocalWeatherViewModel::CheckingPermission,
            LocalWeather::LocationDisabled => LocalWeatherViewModel::LocationDisabled,
            LocalWeather::FetchingLocation => LocalWeatherViewModel::FetchingLocation,
            LocalWeather::FetchingWeather(_) => LocalWeatherViewModel::FetchingWeather,
            LocalWeather::Fetched(_, data) => LocalWeatherViewModel::Fetched(data.clone()),
            LocalWeather::Failed(_) => LocalWeatherViewModel::Failed,
        }
    }
}

impl From<&FavoriteWeather> for FavoriteWeatherViewModel {
    fn from(fw: &FavoriteWeather) -> Self {
        FavoriteWeatherViewModel {
            name: fw.favorite.name().to_string(),
            location: fw.favorite.location(),
            weather: (&fw.weather).into(),
        }
    }
}

impl From<&FavoriteWeatherState> for FavoriteWeatherStateViewModel {
    fn from(state: &FavoriteWeatherState) -> Self {
        match state {
            FavoriteWeatherState::Fetching => FavoriteWeatherStateViewModel::Fetching,
            FavoriteWeatherState::Fetched(data) => {
                FavoriteWeatherStateViewModel::Fetched(data.clone())
            }
            FavoriteWeatherState::Failed => FavoriteWeatherStateViewModel::Failed,
        }
    }
}
