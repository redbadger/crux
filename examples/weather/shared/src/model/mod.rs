pub mod active;
pub mod initializing;
pub mod onboard;
pub(crate) mod outcome;
pub(crate) mod versioned_input;

use crux_core::Command;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::Effect;

pub use self::active::ActiveEvent;
pub use self::active::ActiveModel;
pub use self::initializing::InitializingEvent;
use self::initializing::InitializingModel;
pub use self::onboard::OnboardEvent;

use self::onboard::OnboardModel;

// ANCHOR: event
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum Event {
    Start,
    Onboard(OnboardEvent),
    Active(ActiveEvent),
    #[serde(skip)]
    #[facet(skip)]
    Initializing(InitializingEvent),
}
// ANCHOR_END: event

// ANCHOR: model
#[derive(Default, Debug)]
pub enum Model {
    #[default]
    Uninitialized,
    Initializing(InitializingModel),
    Onboard(OnboardModel),
    Active(ActiveModel),
    Failed(String),
}
// ANCHOR_END: model

impl Model {
    // ANCHOR: update
    pub fn update(&mut self, event: Event) -> Command<Effect, Event> {
        match event {
            Event::Start => {
                let (initializing, cmd) = InitializingModel::start().into_parts();
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
            outcome::Status::Complete(initializing::InitializingTransition::Onboard(favorites)) => {
                *self = Model::Onboard(OnboardModel::new(
                    onboard::OnboardReason::default(),
                    favorites,
                ));
                command
            }
            outcome::Status::Complete(initializing::InitializingTransition::Active(
                api_key,
                favorites,
            )) => {
                let (home_screen, start_cmd) = active::home::HomeScreen::start(favorites, &api_key)
                    .map_event(|he| Event::Active(ActiveEvent::home(he)))
                    .into_parts();
                *self = Model::Active(ActiveModel {
                    api_key,
                    screen: active::Screen::Home(home_screen),
                });
                command.and(start_cmd)
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
            outcome::Status::Complete(onboard::OnboardTransition::Active(api_key, favorites)) => {
                let (home_screen, start_cmd) = active::home::HomeScreen::start(favorites, &api_key)
                    .map_event(|he| Event::Active(ActiveEvent::home(he)))
                    .into_parts();
                *self = Model::Active(ActiveModel {
                    api_key,
                    screen: active::Screen::Home(home_screen),
                });
                command.and(start_cmd)
            }
            outcome::Status::Complete(onboard::OnboardTransition::Failed(msg)) => {
                *self = Model::Failed(msg);
                command
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
            outcome::Status::Complete(active::ActiveTransition::ResetApiKey(favorites)) => {
                *self = Model::Onboard(OnboardModel::new(onboard::OnboardReason::Reset, favorites));
                command
            }
            outcome::Status::Complete(active::ActiveTransition::Unauthorized(favorites)) => {
                *self = Model::Onboard(OnboardModel::new(
                    onboard::OnboardReason::Unauthorized,
                    favorites,
                ));
                command
            }
        }
    }
}

/// The API key used to authenticate to the weather service.
#[derive(Clone, Default, Debug, PartialEq)]
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
