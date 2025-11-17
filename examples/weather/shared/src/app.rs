use crux_core::{
    Command,
    macros::effect,
    render::{RenderOperation, render},
};
use crux_http::protocol::HttpRequest;
use crux_kv::KeyValueOperation;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    favorites::{
        self,
        events::FavoritesEvent,
        model::{Favorite, Favorites, FavoritesState},
    },
    location::{
        Location, capability::LocationOperation, model::geocoding_response::GeocodingResponse,
    },
    weather::{self, events::WeatherEvent, model::current_response::CurrentResponse},
};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum Event {
    Navigate(Box<Workflow>),
    Home(Box<WeatherEvent>),
    Favorites(Box<FavoritesEvent>),
}

#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
    KeyValue(KeyValueOperation),
    Http(HttpRequest),
    Location(LocationOperation),
}

#[derive(Facet, Default, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
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
    pub favorites: Favorites,
    pub search_results: Option<Vec<GeocodingResponse>>,
    pub location_enabled: bool,
    pub last_location: Option<Location>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ViewModel {
    pub workflow: WorkflowViewModel,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum WorkflowViewModel {
    Home {
        weather_data: Box<CurrentResponse>,
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
    name: String,
    location: Location,
    current: Box<Option<CurrentResponse>>,
}

impl From<&Favorite> for FavoriteView {
    fn from(value: &Favorite) -> Self {
        FavoriteView {
            name: value.geo.name.clone(),
            location: (&value.geo).into(),
            current: Box::new(value.current.clone()),
        }
    }
}

#[derive(Default)]
pub struct App;

impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Self::Event, model: &mut Self::Model) -> Command<Effect, Event> {
        // If this is the first update and we're on Home, trigger weather fetch
        match event {
            Event::Navigate(page) => {
                model.page = *page;
                render()
            }
            Event::Home(home_event) => {
                let fav_cmd = if let WeatherEvent::Show = *home_event {
                    favorites::events::update(FavoritesEvent::Restore, model)
                        .map_event(|fe| Event::Favorites(Box::new(fe)))
                } else {
                    Command::done()
                };

                let weather_cmd = weather::events::update(*home_event, model)
                    .map_event(|we| Event::Home(Box::new(we)));

                fav_cmd.and(weather_cmd)
            }

            Event::Favorites(fav_event) => favorites::events::update(*fav_event, model)
                .map_event(|e| Event::Favorites(Box::new(e))),
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        let favorites = model.favorites.iter().map(From::from).collect();

        let workflow = match &model.page {
            Workflow::Home => WorkflowViewModel::Home {
                weather_data: Box::new(model.weather_data.clone()),
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
        );

        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::Idle)
        ));

        // Navigate to Home
        let _ = app.update(Event::Navigate(Box::new(Workflow::Home)), &mut model);
        assert!(matches!(model.page, Workflow::Home));

        // Navigate to AddFavorite
        let _ = app.update(Event::Navigate(Box::new(Workflow::AddFavorite)), &mut model);

        assert!(matches!(model.page, Workflow::AddFavorite));
    }
}
