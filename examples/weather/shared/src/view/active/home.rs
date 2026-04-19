use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    effects::{http::weather::model::current_response::CurrentWeatherResponse, location::Location},
    model::active::home::{FavoriteWeather, FavoriteWeatherState, HomeScreen, LocalWeather},
};

/// View model for the home screen.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct HomeViewModel {
    /// The state of the "weather at my location" workflow.
    pub local_weather: LocalWeatherViewModel,
    /// One entry per saved favourite, in insertion order.
    pub favorites: Vec<FavoriteWeatherViewModel>,
}

/// The local-weather workflow state, flattened for the shell.
///
/// Mirrors [`LocalWeather`] but drops the intermediate `Location` values
/// the UI doesn't need.
///
/// The [`Default`] variant is `CheckingPermission` — the state before any
/// work has been done. Used as the fallback when the web shell projects the
/// top-level view model into a `Memo<HomeViewModel>`.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[repr(C)]
pub enum LocalWeatherViewModel {
    /// Asking the shell whether location services are enabled.
    #[default]
    CheckingPermission,
    /// Location services are off or denied; show a "location disabled" UI.
    LocationDisabled,
    /// Coordinates are being fetched from the shell.
    FetchingLocation,
    /// Coordinates are in hand; the weather API call is in flight.
    FetchingWeather,
    /// Weather received and ready to display.
    Fetched(Box<CurrentWeatherResponse>),
    /// The weather fetch failed (network or parse error); show retry UI.
    Failed,
}

/// One row in the favourites list on the home screen.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FavoriteWeatherViewModel {
    pub name: String,
    pub location: Location,
    pub weather: FavoriteWeatherStateViewModel,
}

/// The fetch state for a single favourite's weather.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum FavoriteWeatherStateViewModel {
    /// Weather request in flight.
    Fetching,
    /// Weather received.
    Fetched(Box<CurrentWeatherResponse>),
    /// Weather fetch failed for this favourite.
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
