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
/// The top-level event type, namespaced by lifecycle stage.
///
/// `Start` kicks the app out of `Uninitialized`. The remaining variants carry
/// sub-events for the stage currently in progress. `Initializing` is internal
/// to the core and not visible to the shell.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum Event {
    /// Sent by the shell once, at launch. Triggers initialisation.
    Start,
    /// Sub-events for the onboarding flow.
    Onboard(OnboardEvent),
    /// Sub-events for the active app (home and favourites).
    Active(ActiveEvent),
    /// Internal events resolving the parallel initialisation fetches.
    #[serde(skip)]
    #[facet(skip)]
    Initializing(InitializingEvent),
}
// ANCHOR_END: event

// ANCHOR: model
/// The app's top-level lifecycle state machine.
///
/// The app moves between mutually exclusive phases: it starts uninitialised,
/// fetches stored data during initialisation, then either onboards the user
/// (if no API key is stored) or activates. From active, a 401 response or a
/// user-initiated reset sends it back to onboarding.
#[derive(Default, Debug)]
pub enum Model {
    /// The default state before the shell sends `Event::Start`. The core
    /// exists but has not begun any work yet.
    #[default]
    Uninitialized,
    /// Shell sent `Event::Start`; fetching the API key and favourites in
    /// parallel.
    Initializing(InitializingModel),
    /// No API key available; prompting the user for one. Entered on first
    /// run, after a 401, or on explicit reset.
    Onboard(OnboardModel),
    /// API key and favourites loaded; running the main app.
    Active(ActiveModel),
    /// Unrecoverable error; carrying a message for the UI.
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

    // ANCHOR: lifecycle_transition
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
                let (onboard, start_cmd) =
                    OnboardModel::start(onboard::OnboardReason::default(), favorites)
                        .map_event(Event::Onboard)
                        .into_parts();
                *self = Model::Onboard(onboard);
                command.and(start_cmd)
            }
            outcome::Status::Complete(initializing::InitializingTransition::Active(
                api_key,
                favorites,
            )) => {
                let (active, start_cmd) = ActiveModel::start(api_key, favorites)
                    .map_event(Event::Active)
                    .into_parts();
                *self = Model::Active(active);
                command.and(start_cmd)
            }
        }
    }
    // ANCHOR_END: lifecycle_transition

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
                let (active, start_cmd) = ActiveModel::start(api_key, favorites)
                    .map_event(Event::Active)
                    .into_parts();
                *self = Model::Active(active);
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
                let (onboard, start_cmd) =
                    OnboardModel::start(onboard::OnboardReason::Reset, favorites)
                        .map_event(Event::Onboard)
                        .into_parts();
                *self = Model::Onboard(onboard);
                command.and(start_cmd)
            }
            outcome::Status::Complete(active::ActiveTransition::Unauthorized(favorites)) => {
                let (onboard, start_cmd) =
                    OnboardModel::start(onboard::OnboardReason::Unauthorized, favorites)
                        .map_event(Event::Onboard)
                        .into_parts();
                *self = Model::Onboard(onboard);
                command.and(start_cmd)
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
