pub mod active;
pub mod settings;
pub mod initializing;
pub(crate) mod outcome;

use crux_core::Command;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::{
    Effect, secret,
    location::Location,
};

use self::active::location::GeocodingResponse;
pub use self::active::ActiveEvent;
pub use self::settings::SettingsEvent;
pub use self::initializing::InitializingEvent;

use self::{
    settings::SettingsModel,
    active::{
        favorites::{
            model::{Favorites, FavoritesState},
        },
        weather::{
            events::WeatherEvent,
            model::current_response::CurrentWeatherResponse,
        },
    },
};

// ANCHOR: event
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum Event {
    Start,
    Initializing(InitializingEvent),
    Settings(SettingsEvent),
    Active(ActiveEvent),
}
// ANCHOR_END: event

// ANCHOR: workflow
#[derive(Facet, Default, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum Workflow {
    #[default]
    Home,
    Favorites(FavoritesState),
    AddFavorite,
}
// ANCHOR_END: workflow

// ANCHOR: model
#[derive(Default, Debug)]
pub enum Model {
    #[default]
    Uninitialized,
    Initializing,
    Settings(SettingsModel),
    Active(ActiveModel),
}

#[derive(Default, Debug)]
pub struct ActiveModel {
    pub api_key: String,
    pub weather_data: CurrentWeatherResponse,
    pub workflow: Workflow,
    pub favorites: Favorites,
    pub search_results: Option<Vec<GeocodingResponse>>,
    pub location_enabled: bool,
    pub last_location: Option<Location>,
}
// ANCHOR_END: model

impl Model {
    // ANCHOR: update
    pub fn update(&mut self, event: Event) -> Command<Effect, Event> {
        match event {
            Event::Start => {
                *self = Model::Initializing;
                secret::command::fetch(secret::API_KEY_NAME).then_send(|r| {
                    Event::Initializing(InitializingEvent::SecretFetched(r))
                })
            }
            Event::Initializing(event) => initializing::update(event, self),
            Event::Settings(event) => self.update_settings(event),
            Event::Active(event) => self.update_active(event),
        }
    }
    // ANCHOR_END: update

    fn update_settings(&mut self, event: SettingsEvent) -> Command<Effect, Event> {
        let owned = std::mem::take(self);
        let Model::Settings(config) = owned else {
            *self = owned;
            return Command::done();
        };
        match config.update(event).map_event(Event::Settings) {
            outcome::Outcome::Continue(config, command) => {
                *self = Model::Settings(config);
                command
            }
            outcome::Outcome::Complete(api_key, command) => {
                *self = Model::Active(ActiveModel {
                    api_key,
                    ..Default::default()
                });
                command.and(Command::event(Event::Active(ActiveEvent::Home(Box::new(
                    WeatherEvent::Show,
                )))))
            }
        }
    }

    fn update_active(&mut self, event: ActiveEvent) -> Command<Effect, Event> {
        let owned = std::mem::take(self);
        let Model::Active(active_model) = owned else {
            *self = owned;
            return Command::done();
        };
        match active_model.update(event).map_event(Event::Active) {
            outcome::Outcome::Continue(active_model, command) => {
                *self = Model::Active(active_model);
                command
            }
            outcome::Outcome::Complete(active::ActiveTransition::ResetApiKey, command) => {
                *self = Model::Settings(SettingsModel::default());
                command
            }
        }
    }
}
