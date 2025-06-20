use crux_core::{
    macros::effect,
    render::{render, RenderOperation},
    Command,
};
use crux_http::protocol::HttpRequest;
use crux_kv::KeyValueOperation;
use serde::{Deserialize, Serialize};

use crate::{
    favorites::{
        self,
        events::FavoritesEvent,
        model::{Favorite, FavoritesState},
    },
    location::{
        capability::{LocationOperation, LocationResponse},
        model::geocoding_response::GeocodingResponse,
    },
    weather::{self, events::WeatherEvent, model::current_response::CurrentResponse},
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Event {
    Navigate(Box<Workflow>),
    Home(Box<WeatherEvent>),
    Favorites(Box<FavoritesEvent>),
}

#[effect(typegen)]
pub enum Effect {
    Render(RenderOperation),
    KeyValue(KeyValueOperation),
    Http(HttpRequest),
    Location(LocationOperation),
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Workflow {
    #[default]
    Home,
    Favorites(FavoritesState),
    AddFavorite,
}

#[derive(Default, Debug)]
pub struct Model {
    pub weather_data: CurrentResponse,
    pub page: Workflow,
    pub favorites: Vec<Favorite>,
    pub search_results: Option<Vec<GeocodingResponse>>,
    pub location_enabled: bool,
    pub last_location: Option<LocationResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ViewModel {
    pub workflow: WorkflowViewModel,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum WorkflowViewModel {
    Home {
        weather_data: Box<CurrentResponse>,
        favorites: Vec<FavoriteView>,
    },
    Favorites {
        favorites: Vec<FavoriteView>,
    },
    AddFavorite {
        search_results: Option<Vec<GeocodingResponse>>,
    },
    ConfirmDeleteFavorite {
        lat: f64,
        lng: f64,
        favorites: Vec<FavoriteView>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FavoriteView {
    name: String,
    lat: f64,
    lon: f64,
    current: Box<Option<CurrentResponse>>,
}

#[derive(Default)]
pub struct App;

impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Capabilities = (); // will be deprecated, so use unit type for now
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        _caps: &(),
    ) -> Command<Effect, Event> {
        // If this is the first update and we're on Home, trigger weather fetch
        match event {
            Event::Navigate(page) => {
                model.page = *page;
                render()
            }
            Event::Home(home_event) => weather::events::update(*home_event, model),
            Event::Favorites(fav_event) => favorites::events::update(*fav_event, model)
                .map_event(|e| Event::Favorites(Box::new(e))),
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        let favorites = model
            .favorites
            .iter()
            .map(|f| FavoriteView {
                name: f.geo.name.clone(),
                lat: f.geo.lat,
                lon: f.geo.lon,
                current: Box::new(f.current.clone()),
            })
            .collect();

        let workflow = match &model.page {
            Workflow::Home => WorkflowViewModel::Home {
                weather_data: Box::new(model.weather_data.clone()),
                favorites,
            },
            Workflow::Favorites(FavoritesState::Idle) => WorkflowViewModel::Favorites { favorites },
            Workflow::Favorites(FavoritesState::ConfirmDelete(lat, lng)) => {
                WorkflowViewModel::ConfirmDeleteFavorite {
                    lat: *lat,
                    lng: *lng,
                    favorites,
                }
            }
            Workflow::AddFavorite => WorkflowViewModel::AddFavorite {
                search_results: model.search_results.clone(),
            },
        };

        ViewModel { workflow }
    }
}

#[cfg(test)]
mod tests {
    use crux_core::App as _;

    use super::*;

    #[test]
    fn test_navigation() {
        let app = App;
        let mut model = Model::default();

        // Navigate to Favorites
        let _ = app.update(
            Event::Navigate(Box::new(Workflow::Favorites(FavoritesState::Idle))),
            &mut model,
            &(),
        );

        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::Idle)
        ));

        // Navigate to Home
        let _ = app.update(Event::Navigate(Box::new(Workflow::Home)), &mut model, &());
        assert!(matches!(model.page, Workflow::Home));

        // Navigate to AddFavorite
        let _ = app.update(
            Event::Navigate(Box::new(Workflow::AddFavorite)),
            &mut model,
            &(),
        );

        assert!(matches!(model.page, Workflow::AddFavorite));
    }
}
