//! Sets the user up to use the application for the first time, by entering their API key.
//!
//! Triggered by: missing api key in secrets during `InitializingModel`
//! Outcome: the `ApiKey` defined and submitted by the user.

use crux_core::{render::render, Command};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::secret::{self, SecretFetchResponse, SecretStoreResponse};

use super::{ApiKey, outcome::Outcome};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum OnboardEvent {
    ApiKey(String),
    Submit,

    #[serde(skip)]
    #[facet(skip)]
    SecretStored(#[facet(opaque)] SecretStoreResponse),

    #[serde(skip)]
    #[facet(skip)]
    SecretFetched(#[facet(opaque)] SecretFetchResponse),
}

/// Model for the onboarding screen where the user enters their API key.
#[derive(Debug)]
pub struct OnboardModel {
    pub api_key_input: String,
    pub error: Option<String>,
}

impl Default for OnboardModel {
    fn default() -> Self {
        Self {
            api_key_input: String::new(),
            error: None,
        }
    }
}

impl OnboardModel {
    /// Process an onboard event. Returns `Complete(api_key)` when the key
    /// has been stored and confirmed, or `Continue(self)` to stay on this screen.
    pub(crate) fn update(mut self, event: OnboardEvent) -> Outcome<Self, ApiKey, OnboardEvent> {
        match event {
            OnboardEvent::ApiKey(text) => {
                self.api_key_input = text;
                Outcome::continuing(self, render())
            }
            OnboardEvent::Submit => {
                let key = self.api_key_input.trim().to_string();
                if key.is_empty() {
                    self.error = Some("API key cannot be empty".to_string());
                    return Outcome::continuing(self, render());
                }
                let cmd = secret::command::store(secret::API_KEY_NAME, key)
                    .then_send(OnboardEvent::SecretStored);
                Outcome::continuing(self, cmd)
            }
            OnboardEvent::SecretStored(response) => match response {
                SecretStoreResponse::Stored(_) => {
                    let cmd = secret::command::fetch(secret::API_KEY_NAME)
                        .then_send(OnboardEvent::SecretFetched);
                    Outcome::continuing(self, cmd)
                }
                SecretStoreResponse::StoreError(msg) => {
                    self.error = Some(format!("Failed to store: {msg}"));
                    Outcome::continuing(self, render())
                }
            },
            OnboardEvent::SecretFetched(response) => match response {
                SecretFetchResponse::Fetched(api_key) => {
                    Outcome::complete(api_key.into(), Command::done())
                }
                SecretFetchResponse::Missing(_) => {
                    self.error = Some("Failed to read back stored key".to_string());
                    Outcome::continuing(self, render())
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::effects::secret::{self, SecretRequest};

    use super::*;

    #[test]
    fn test_set_updates_input() {
        let model = OnboardModel::default();
        let outcome = model.update(OnboardEvent::ApiKey("abc123".to_string()));

        let (model, mut cmd) = outcome.expect_continue().into_parts();
        assert_eq!(model.api_key_input, "abc123");
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn test_submit_stores_secret() {
        let model = OnboardModel {
            api_key_input: "my_new_key".to_string(),
            error: None,
        };
        let outcome = model.update(OnboardEvent::Submit);

        let mut cmd = outcome.expect_continue().into_command();
        let request = cmd.expect_one_effect().expect_secret();
        assert_eq!(
            request.operation,
            SecretRequest::Store(secret::API_KEY_NAME.to_string(), "my_new_key".to_string(),)
        );
    }

    #[test]
    fn test_submit_empty_shows_error() {
        let model = OnboardModel {
            api_key_input: "  ".to_string(),
            error: None,
        };
        let outcome = model.update(OnboardEvent::Submit);

        let (model, mut cmd) = outcome.expect_continue().into_parts();
        cmd.expect_one_effect().expect_render();
        assert_eq!(model.error.as_deref(), Some("API key cannot be empty"));
    }

    #[test]
    fn test_store_error_shows_message() {
        let model = OnboardModel::default();
        let outcome = model.update(OnboardEvent::SecretStored(SecretStoreResponse::StoreError(
            "disk full".to_string(),
        )));

        let (model, mut cmd) = outcome.expect_continue().into_parts();
        cmd.expect_one_effect().expect_render();
        assert!(model.error.as_ref().unwrap().contains("disk full"));
    }

    #[test]
    fn test_stored_then_fetched_completes_with_api_key() {
        let model = OnboardModel::default();

        // First: SecretStored -> should re-fetch
        let outcome = model.update(OnboardEvent::SecretStored(SecretStoreResponse::Stored(
            secret::API_KEY_NAME.to_string(),
        )));
        let (model, mut cmd) = outcome.expect_continue().into_parts();
        let request = cmd.expect_one_effect().expect_secret();
        assert_eq!(
            request.operation,
            SecretRequest::Fetch(secret::API_KEY_NAME.to_string())
        );

        // Then: SecretFetched -> should complete
        let outcome = model.update(OnboardEvent::SecretFetched(SecretFetchResponse::Fetched(
            "the_key".to_string(),
        )));
        let api_key = outcome.expect_complete().into_value();
        assert_eq!(api_key, "the_key");
    }
}
