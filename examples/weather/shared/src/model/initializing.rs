//! Initializes the application to run, resulting in either onboarding or an active state.
//!
//! Triggered by: `Event::Start`
//!
//! Fetches the API key and favorites in parallel. Outcome is one of:
//! - `InitializingTransition::Onboard` - no API key, transitions to onboarding.
//! - `InitializingTransition::Active(api_key, favorites)` - fully initialized.

use crux_core::{render::render, Command};
use crux_kv::command::KeyValue;
use crux_kv::error::KeyValueError;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::{
    secret::{self, SecretFetchResponse},
    Effect,
};

use super::{
    ApiKey,
    active::favorites::model::{FAVORITES_KEY, Favorite, Favorites},
    outcome::Outcome,
    Event,
};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum InitializingEvent {
    #[serde(skip)]
    #[facet(skip)]
    SecretFetched(#[facet(opaque)] SecretFetchResponse),

    #[serde(skip)]
    #[facet(skip)]
    FavoritesLoaded(#[facet(opaque)] Result<Option<Vec<u8>>, KeyValueError>),
}

#[derive(Debug)]
pub(crate) enum InitializingTransition {
    Onboard,
    Active(ApiKey, Favorites),
}

#[derive(Default, Debug)]
pub struct InitializingModel {
    api_key: Option<ApiKey>,
    favorites: Option<Favorites>,
}

impl InitializingModel {
    pub fn start() -> (Self, Command<Effect, Event>) {
        tracing::debug!("starting initialization, fetching API key and favorites");

        let fetch_secret = secret::command::fetch(secret::API_KEY_NAME)
            .then_send(|r| Event::Initializing(InitializingEvent::SecretFetched(r)));
        let fetch_favorites = KeyValue::get(FAVORITES_KEY)
            .then_send(|r| Event::Initializing(InitializingEvent::FavoritesLoaded(r)));

        (Self::default(), Command::all([fetch_secret, fetch_favorites]))
    }

    pub(crate) fn update(
        mut self,
        event: InitializingEvent,
    ) -> Outcome<Self, InitializingTransition, InitializingEvent> {
        match event {
            InitializingEvent::SecretFetched(response) => match response {
                SecretFetchResponse::Missing(_) => {
                    tracing::debug!("API key missing, transitioning to onboarding");
                    Outcome::complete(InitializingTransition::Onboard, render())
                }
                SecretFetchResponse::Fetched(api_key) => {
                    tracing::debug!("received API key");
                    self.api_key = Some(api_key.into());
                    self.resolve()
                }
            },
            InitializingEvent::FavoritesLoaded(result) => {
                let favorites = result
                    .ok()
                    .flatten()
                    .and_then(|bytes| serde_json::from_slice::<Vec<Favorite>>(&bytes).ok())
                    .map(Favorites::from_vec)
                    .unwrap_or_default();

                tracing::debug!("loaded {} favorites", favorites.len());
                self.favorites = Some(favorites);
                self.resolve()
            }
        }
    }

    fn resolve(self) -> Outcome<Self, InitializingTransition, InitializingEvent> {
        match (self.api_key, self.favorites) {
            (Some(api_key), Some(favorites)) => {
                tracing::debug!("initialization complete, transitioning to active");
                Outcome::complete(InitializingTransition::Active(api_key, favorites), Command::done())
            }
            (api_key, favorites) => {
                tracing::debug!("waiting for remaining initialization data");
                Outcome::continuing(Self { api_key, favorites }, render())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::secret;

    const API_KEY_VALUE: &str = "my_key";

    #[test]
    fn secret_missing_transitions_to_onboard() {
        let model = InitializingModel::default();
        let outcome = model.update(InitializingEvent::SecretFetched(
            SecretFetchResponse::Missing(secret::API_KEY_NAME.to_string()),
        ));

        let (transition, mut cmd) = outcome.expect_complete().into_parts();
        assert!(matches!(transition, InitializingTransition::Onboard));
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn secret_fetched_continues_waiting_for_favorites() {
        let model = InitializingModel::default();
        let outcome = model.update(InitializingEvent::SecretFetched(
            SecretFetchResponse::Fetched(API_KEY_VALUE.to_string()),
        ));

        let (model, mut cmd) = outcome.expect_continue().into_parts();
        assert!(model.api_key.is_some());
        assert!(model.favorites.is_none());
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn favorites_loaded_continues_waiting_for_secret() {
        let model = InitializingModel::default();
        let outcome = model.update(InitializingEvent::FavoritesLoaded(Ok(None)));

        let (model, mut cmd) = outcome.expect_continue().into_parts();
        assert!(model.api_key.is_none());
        assert!(model.favorites.is_some());
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn both_loaded_transitions_to_active() {
        let mut model = InitializingModel::default();

        // First: secret arrives
        let outcome = model.update(InitializingEvent::SecretFetched(
            SecretFetchResponse::Fetched(API_KEY_VALUE.to_string()),
        ));
        model = outcome.expect_continue().into_value();

        // Second: favorites arrive
        let outcome = model.update(InitializingEvent::FavoritesLoaded(Ok(None)));
        let (transition, _cmd) = outcome.expect_complete().into_parts();

        assert_let_bind::assert_let!(
            InitializingTransition::Active(api_key, favorites),
            transition
        );
        assert_eq!(api_key, API_KEY_VALUE);
        assert!(favorites.is_empty());
    }
}
