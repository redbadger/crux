use crux_core::{
    macros::effect,
    render::{render, RenderOperation},
    Command,
};
use crux_http::protocol::HttpRequest;
use serde::{Deserialize, Serialize};

use crate::{
    events::{self, CurrentWeatherEvent},
    workflows::{
        self,
        favorites::{Favorite, FavoritesState},
        AddFavoriteEvent, FavoritesEvent, HomeEvent,
    },
    CurrentResponse,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Event {
    Navigate(Workflow),
    Home(HomeEvent),
    Favorites(FavoritesEvent),
    AddFavorite(AddFavoriteEvent),

    #[serde(skip)]
    CurrentWeather(CurrentWeatherEvent),
}

#[effect(typegen)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Workflow {
    #[default]
    Home,
    Favorites(FavoritesState),
    AddFavorite,
}

#[derive(Default)]
pub struct Model {
    pub weather_data: CurrentResponse,
    pub page: Workflow,
    pub favorites: Vec<Favorite>,
    pub show_add_modal: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ViewModel {
    pub workflow: WorkflowViewModel,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum WorkflowViewModel {
    Home { weather_data: CurrentResponse },
    Favorites { favorites: Vec<FavoriteView> },
    AddFavorite,
    ConfirmDeleteFavorite { lat: f64, lng: f64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FavoriteView {
    name: String,
    lat: f64,
    lon: f64,
    summary: Option<String>,
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
        match event {
            Event::Navigate(page) => {
                model.page = page;
                render()
            }
            Event::Home(home_event) => workflows::update_home(home_event, model),
            Event::Favorites(fav_event) => workflows::update_favorites(fav_event, model),
            Event::AddFavorite(add_event) => workflows::update_add_favorite(add_event, model),
            Event::CurrentWeather(current_weather_event) => {
                events::update_current_weather(current_weather_event, model)
            }
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        let workflow = match &model.page {
            Workflow::Home => WorkflowViewModel::Home {
                weather_data: model.weather_data.clone(),
            },
            Workflow::Favorites(FavoritesState::Idle) => WorkflowViewModel::Favorites {
                favorites: model
                    .favorites
                    .iter()
                    .map(|f| FavoriteView {
                        name: f.geo.name.clone(),
                        lat: f.geo.lat,
                        lon: f.geo.lon,
                        summary: f.current.as_ref().map(|c| {
                            format!(
                                "{}°C, {}",
                                c.main.temp,
                                c.weather
                                    .get(0)
                                    .map(|w| w.description.clone())
                                    .unwrap_or_default()
                            )
                        }),
                    })
                    .collect(),
            },
            Workflow::Favorites(FavoritesState::ConfirmDelete(lat, lng)) => {
                WorkflowViewModel::ConfirmDeleteFavorite {
                    lat: *lat,
                    lng: *lng,
                }
            }
            Workflow::AddFavorite => WorkflowViewModel::AddFavorite,
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
        let mut cmd = app.update(
            Event::Navigate(Workflow::Favorites(FavoritesState::Idle)),
            &mut model,
            &(),
        );
        assert!(matches!(cmd.effects().next(), Some(Effect::Render(_))));
        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::Idle)
        ));

        // Navigate to Home
        let mut cmd = app.update(Event::Navigate(Workflow::Home), &mut model, &());
        assert!(matches!(cmd.effects().next(), Some(Effect::Render(_))));
        assert!(matches!(model.page, Workflow::Home));

        // Navigate to AddFavorite
        let mut cmd = app.update(Event::Navigate(Workflow::AddFavorite), &mut model, &());
        assert!(matches!(cmd.effects().next(), Some(Effect::Render(_))));
        assert!(matches!(model.page, Workflow::AddFavorite));
    }
}
