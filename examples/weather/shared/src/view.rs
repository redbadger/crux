use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    effects::{
        location::Location,
        http::{
            location::GeocodingResponse,
            weather::model::current_response::CurrentWeatherResponse,
        },
    },
    model::{
        Model, onboard,
        active::{
            Screen,
            favorites::{FavoritesWorkflow, model::Favorite},
            home::{FavoriteWeather, FavoriteWeatherState, LocalWeather},
        },
    },
};

// ANCHOR: view_model
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ViewModel {
    pub workflow: WorkflowViewModel,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum WorkflowViewModel {
    Loading,
    Onboard {
        reason: onboard::OnboardReason,
        api_key_input: String,
        can_submit: bool,
        saving: bool,
    },
    Home {
        weather_data: Option<Box<CurrentWeatherResponse>>,
        favorites: Vec<FavoriteView>,
    },
    Favorites {
        favorites: Vec<FavoriteView>,
        delete_confirmation: Option<Location>,
    },
    AddFavorite {
        search_results: Option<Vec<GeocodingResponse>>,
        searching: bool,
    },
    Failed {
        message: String,
    },
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FavoriteView {
    pub name: String,
    pub location: Location,
    pub current: Box<Option<CurrentWeatherResponse>>,
}
// ANCHOR_END: view_model

impl From<&Favorite> for FavoriteView {
    fn from(value: &Favorite) -> Self {
        FavoriteView {
            name: value.name().to_string(),
            location: value.location(),
            current: Box::new(None),
        }
    }
}

impl From<&FavoriteWeather> for FavoriteView {
    fn from(value: &FavoriteWeather) -> Self {
        let current = match &value.weather {
            FavoriteWeatherState::Fetched(data) => Some(data.as_ref().clone()),
            _ => None,
        };
        FavoriteView {
            name: value.favorite.name().to_string(),
            location: value.favorite.location(),
            current: Box::new(current),
        }
    }
}

// ANCHOR: view
impl From<&Model> for ViewModel {
    fn from(model: &Model) -> Self {
        let workflow = match model {
            Model::Uninitialized | Model::Initializing(_) => WorkflowViewModel::Loading,
            Model::Onboard(onboard) => WorkflowViewModel::Onboard {
                reason: onboard.reason,
                can_submit: onboard.can_submit(),
                api_key_input: match &onboard.state {
                    onboard::OnboardState::Input { api_key } => api_key.clone(),
                    onboard::OnboardState::Saving { .. } => String::new(),
                },
                saving: matches!(onboard.state, onboard::OnboardState::Saving { .. }),
            },
            Model::Active(active) => match &active.screen {
                Screen::Home(home) => {
                    let weather_data = match &home.current_weather {
                        LocalWeather::Fetched(_, data) => Some(data.clone()),
                        _ => None,
                    };
                    let favorites = home.favorites_weather.iter().map(From::from).collect();
                    WorkflowViewModel::Home {
                        weather_data,
                        favorites,
                    }
                }
                Screen::Favorites(fav) => match &fav.workflow {
                    None => {
                        let favorites = fav.favorites.iter().map(From::from).collect();
                        WorkflowViewModel::Favorites {
                            favorites,
                            delete_confirmation: None,
                        }
                    }
                    Some(FavoritesWorkflow::ConfirmDelete(wf)) => {
                        let favorites = fav.favorites.iter().map(From::from).collect();
                        WorkflowViewModel::Favorites {
                            favorites,
                            delete_confirmation: Some(wf.location),
                        }
                    }
                    Some(FavoritesWorkflow::Add(wf)) => {
                        WorkflowViewModel::AddFavorite {
                            search_results: wf.search_results.clone(),
                            searching: wf.searching,
                        }
                    }
                },
            }
            Model::Failed(message) => WorkflowViewModel::Failed {
                message: message.clone(),
            },
        };

        ViewModel { workflow }
    }
}
// ANCHOR_END: view
