pub mod active;
pub mod initializing;
pub mod onboard;
pub(crate) mod outcome;

use crux_core::Command;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::{location::Location, Effect};

use self::active::location::GeocodingResponse;
pub use self::active::ActiveEvent;
pub use self::initializing::InitializingEvent;
use self::initializing::InitializingModel;
pub use self::onboard::OnboardEvent;

use self::{
    active::{
        favorites::model::{Favorites, FavoritesState},
        weather::{events::WeatherEvent, model::current_response::CurrentWeatherResponse},
    },
    onboard::OnboardModel,
};

// ANCHOR: event
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum Event {
    Start,
    Initializing(InitializingEvent),
    Onboard(OnboardEvent),
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
    Initializing(InitializingModel),
    Onboard(OnboardModel),
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
                let (initializing, cmd) = InitializingModel::start();
                *self = Model::Initializing(initializing);
                cmd
            }
            Event::Initializing(event) => self.update_initializing(event),
            Event::Onboard(event) => self.update_onboard(event),
            Event::Active(event) => self.update_active(event),
        }
    }
    // ANCHOR_END: update

    fn update_initializing(&mut self, event: InitializingEvent) -> Command<Effect, Event> {
        let owned = std::mem::take(self);
        let Model::Initializing(initializing) = owned else {
            *self = owned;
            return Command::done();
        };
        let (status, command) = initializing
            .update(event)
            .map_event(Event::Initializing)
            .into_parts();
        match status {
            outcome::Status::Continue(initializing) => {
                *self = Model::Initializing(initializing);
                command
            }
            outcome::Status::Complete(initializing::InitializingTransition::Onboard) => {
                *self = Model::Onboard(OnboardModel::default());
                command
            }
            outcome::Status::Complete(initializing::InitializingTransition::Active(api_key)) => {
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

    fn update_onboard(&mut self, event: OnboardEvent) -> Command<Effect, Event> {
        let owned = std::mem::take(self);
        let Model::Onboard(config) = owned else {
            *self = owned;
            return Command::done();
        };
        let (status, command) = config.update(event).map_event(Event::Onboard).into_parts();
        match status {
            outcome::Status::Continue(config) => {
                *self = Model::Onboard(config);
                command
            }
            outcome::Status::Complete(api_key) => {
                *self = Model::Active(ActiveModel {
                    api_key: api_key.into(),
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
        let (status, command) = active_model
            .update(event)
            .map_event(Event::Active)
            .into_parts();
        match status {
            outcome::Status::Continue(active_model) => {
                *self = Model::Active(active_model);
                command
            }
            outcome::Status::Complete(active::ActiveTransition::ResetApiKey) => {
                *self = Model::Onboard(OnboardModel::default());
                command
            }
        }
    }
}

/// The API key used to authenticate to the weather service.
#[derive(Debug, PartialEq)]
pub struct ApiKey(String);

impl From<String> for ApiKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl PartialEq<&str> for ApiKey {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl From<ApiKey> for String {
    fn from(key: ApiKey) -> Self {
        key.0
    }
}
