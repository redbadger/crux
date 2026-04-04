use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    effects::location::Location,
    model::{
        Model, Workflow,
        active::{
            favorites::model::{Favorite, FavoritesState},
            location::GeocodingResponse,
            weather::model::current_response::CurrentWeatherResponse,
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
        api_key_input: String,
        error: Option<String>,
    },
    Home {
        weather_data: Box<CurrentWeatherResponse>,
        favorites: Vec<FavoriteView>,
    },
    Favorites {
        favorites: Vec<FavoriteView>,
        delete_confirmation: Option<Location>,
    },
    AddFavorite {
        search_results: Option<Vec<GeocodingResponse>>,
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
            name: value.geo.name.clone(),
            location: (&value.geo).into(),
            current: Box::new(value.current.clone()),
        }
    }
}

// ANCHOR: view
impl From<&Model> for ViewModel {
    fn from(model: &Model) -> Self {
        let workflow = match model {
            Model::Uninitialized | Model::Initializing(_) => WorkflowViewModel::Loading,
            Model::Onboard(config) => WorkflowViewModel::Onboard {
                api_key_input: config.api_key_input.clone(),
                error: config.error.clone(),
            },
            Model::Active(active) => {
                let favorites = active.favorites.iter().map(From::from).collect();

                match &active.workflow {
                    Workflow::Home => WorkflowViewModel::Home {
                        weather_data: Box::new(active.weather_data.clone()),
                        favorites,
                    },
                    Workflow::Favorites(favorites_state) => match favorites_state {
                        FavoritesState::Idle => WorkflowViewModel::Favorites {
                            favorites,
                            delete_confirmation: None,
                        },
                        FavoritesState::ConfirmDelete(location) => WorkflowViewModel::Favorites {
                            favorites,
                            delete_confirmation: Some(*location),
                        },
                    },
                    Workflow::AddFavorite => WorkflowViewModel::AddFavorite {
                        search_results: active.search_results.clone(),
                    },
                }
            }
        };

        ViewModel { workflow }
    }
}
// ANCHOR_END: view
