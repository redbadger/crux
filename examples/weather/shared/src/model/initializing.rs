//! Initializes the application to run, resulting in either onboarding or an active state.
//!
//! Triggered by: `Event::Start`
//!
//! Fetches the API key and favorites in parallel. Outcome is one of:
//! - `InitializingTransition::Onboard` - no API key, transitions to onboarding.
//! - `InitializingTransition::Active(api_key, favorites)` - fully initialized.

use crux_core::{Command, render::render};
use crux_kv::command::KeyValue;
use crux_kv::error::KeyValueError;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::secret::{self, SecretFetchResponse};

use super::{
    ApiKey, Event,
    active::favorites::model::{FAVORITES_KEY, Favorite, Favorites},
    outcome::{Outcome, Started},
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
    Onboard(Favorites),
    Active(ApiKey, Favorites),
}

#[derive(Default, Debug)]
enum InitializingValue<T> {
    #[default]
    Fetching,
    Fetched(T),
}

#[derive(Default, Debug)]
pub struct InitializingModel {
    api_key: InitializingValue<Option<ApiKey>>,
    favorites: InitializingValue<Favorites>,
}

impl InitializingModel {
    pub(crate) fn start() -> Started<Self, Event> {
        tracing::debug!("starting initialization, fetching API key and favorites");

        let fetch_secret = secret::command::fetch(secret::API_KEY_NAME)
            .then_send(|r| Event::Initializing(InitializingEvent::SecretFetched(r)));
        let fetch_favorites = KeyValue::get(FAVORITES_KEY)
            .then_send(|r| Event::Initializing(InitializingEvent::FavoritesLoaded(r)));

        Started::new(
            Self::default(),
            Command::all([fetch_secret, fetch_favorites]),
        )
    }

    pub(crate) fn update(
        mut self,
        event: InitializingEvent,
    ) -> Outcome<Self, InitializingTransition, InitializingEvent> {
        match event {
            InitializingEvent::SecretFetched(response) => {
                let api_key = match response {
                    SecretFetchResponse::Missing(_) => {
                        tracing::debug!("API key missing");
                        None
                    }
                    SecretFetchResponse::Fetched(api_key) => {
                        tracing::debug!("received API key");
                        Some(api_key.into())
                    }
                };
                self.api_key = InitializingValue::Fetched(api_key);
                self.resolve()
            }
            InitializingEvent::FavoritesLoaded(result) => {
                let favorites = result
                    .ok()
                    .flatten()
                    .and_then(|bytes| serde_json::from_slice::<Vec<Favorite>>(&bytes).ok())
                    .map(Favorites::from_vec)
                    .unwrap_or_default();

                tracing::debug!("loaded {} favorites", favorites.len());
                self.favorites = InitializingValue::Fetched(favorites);
                self.resolve()
            }
        }
    }

    fn resolve(self) -> Outcome<Self, InitializingTransition, InitializingEvent> {
        match (self.api_key, self.favorites) {
            (InitializingValue::Fetched(Some(api_key)), InitializingValue::Fetched(favorites)) => {
                tracing::debug!("initialization complete, transitioning to active");
                Outcome::complete(
                    InitializingTransition::Active(api_key, favorites),
                    Command::done(),
                )
            }
            (InitializingValue::Fetched(None), InitializingValue::Fetched(favorites)) => {
                tracing::debug!("API key missing, transitioning to onboarding");
                Outcome::complete(InitializingTransition::Onboard(favorites), render())
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
    fn secret_missing_continues_waiting_for_favorites() {
        let model = InitializingModel::default();
        let outcome = model.update(InitializingEvent::SecretFetched(
            SecretFetchResponse::Missing(secret::API_KEY_NAME.to_string()),
        ));

        let (model, mut cmd) = outcome.expect_continue().into_parts();
        assert!(matches!(model.api_key, InitializingValue::Fetched(None)));
        assert!(matches!(model.favorites, InitializingValue::Fetching));
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn secret_missing_then_favorites_transitions_to_onboard() {
        let mut model = InitializingModel::default();

        // First: secret is missing
        let outcome = model.update(InitializingEvent::SecretFetched(
            SecretFetchResponse::Missing(secret::API_KEY_NAME.to_string()),
        ));
        model = outcome.expect_continue().into_value();

        // Second: favorites arrive
        let outcome = model.update(InitializingEvent::FavoritesLoaded(Ok(None)));
        let (transition, mut cmd) = outcome.expect_complete().into_parts();
        assert!(matches!(transition, InitializingTransition::Onboard(_)));
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn secret_fetched_continues_waiting_for_favorites() {
        let model = InitializingModel::default();
        let outcome = model.update(InitializingEvent::SecretFetched(
            SecretFetchResponse::Fetched(API_KEY_VALUE.to_string()),
        ));

        let (model, mut cmd) = outcome.expect_continue().into_parts();
        assert!(matches!(model.api_key, InitializingValue::Fetched(Some(_))));
        assert!(matches!(model.favorites, InitializingValue::Fetching));
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn favorites_loaded_continues_waiting_for_secret() {
        let model = InitializingModel::default();
        let outcome = model.update(InitializingEvent::FavoritesLoaded(Ok(None)));

        let (model, mut cmd) = outcome.expect_continue().into_parts();
        assert!(matches!(model.api_key, InitializingValue::Fetching));
        assert!(matches!(model.favorites, InitializingValue::Fetched(_)));
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
