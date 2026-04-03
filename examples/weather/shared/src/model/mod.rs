pub mod active;
pub mod configuration;
pub mod favorites;
pub mod initializing;
pub mod location;
pub(crate) mod outcome;
pub mod weather;

use crux_core::Command;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::{
    Effect, secret,
    location::Location,
    secret::{SecretDeleteResponse, SecretFetchResponse, SecretStoreResponse},
};

use self::location::GeocodingResponse;

use self::{
    configuration::ConfigurationModel,
    favorites::{
        events::FavoritesEvent,
        model::{Favorites, FavoritesState},
    },
    weather::{
        events::WeatherEvent,
        model::current_response::CurrentWeatherResponse,
    },
};

// ANCHOR: event
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum Event {
    Start,
    Initializing(InitializingEvent),
    Configuration(ConfigurationEvent),
    Active(ActiveEvent),
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum InitializingEvent {
    #[serde(skip)]
    #[facet(skip)]
    SecretFetched(#[facet(opaque)] SecretFetchResponse),
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum ConfigurationEvent {
    Set(String),
    Submit,

    #[serde(skip)]
    #[facet(skip)]
    SecretStored(#[facet(opaque)] SecretStoreResponse),

    #[serde(skip)]
    #[facet(skip)]
    SecretFetched(#[facet(opaque)] SecretFetchResponse),
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum ActiveEvent {
    Navigate(Box<Workflow>),
    Home(Box<WeatherEvent>),
    Favorites(Box<FavoritesEvent>),
    ResetApiKey,

    #[serde(skip)]
    #[facet(skip)]
    SecretDeleted(#[facet(opaque)] SecretDeleteResponse),
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
    Configuration(ConfigurationModel),
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
            Event::Configuration(event) => self.update_configuration(event),
            Event::Active(event) => self.update_active(event),
        }
    }
    // ANCHOR_END: update

    fn update_configuration(&mut self, event: ConfigurationEvent) -> Command<Effect, Event> {
        let owned = std::mem::take(self);
        let Model::Configuration(config) = owned else {
            *self = owned;
            return Command::done();
        };
        match config.update(event).map_event(Event::Configuration) {
            outcome::Outcome::Continue(config, command) => {
                *self = Model::Configuration(config);
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
                *self = Model::Configuration(ConfigurationModel::default());
                command
            }
        }
    }
}
