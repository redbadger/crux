use crux_core::{Command, render::render};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::secret::{self, SecretFetchResponse, SecretStoreResponse};

use super::outcome::Outcome;

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum OnboardEvent {
    Set(String),
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
    pub(crate) fn update(
        mut self,
        event: OnboardEvent,
    ) -> Outcome<Self, String, OnboardEvent> {
        match event {
            OnboardEvent::Set(text) => {
                self.api_key_input = text;
                Outcome::Continue(self, render())
            }
            OnboardEvent::Submit => {
                let key = self.api_key_input.trim().to_string();
                if key.is_empty() {
                    self.error = Some("API key cannot be empty".to_string());
                    return Outcome::Continue(self, render());
                }
                let cmd = secret::command::store(secret::API_KEY_NAME, key)
                    .then_send(OnboardEvent::SecretStored);
                Outcome::Continue(self, cmd)
            }
            OnboardEvent::SecretStored(response) => match response {
                SecretStoreResponse::Stored(_) => {
                    let cmd = secret::command::fetch(secret::API_KEY_NAME)
                        .then_send(OnboardEvent::SecretFetched);
                    Outcome::Continue(self, cmd)
                }
                SecretStoreResponse::StoreError(msg) => {
                    self.error = Some(format!("Failed to store: {msg}"));
                    Outcome::Continue(self, render())
                }
            },
            OnboardEvent::SecretFetched(response) => match response {
                SecretFetchResponse::Fetched(api_key) => {
                    Outcome::Complete(api_key, Command::done())
                }
                SecretFetchResponse::Missing(_) => {
                    self.error = Some("Failed to read back stored key".to_string());
                    Outcome::Continue(self, render())
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
        let outcome = model.update(OnboardEvent::Set("abc123".to_string()));

        match outcome {
            Outcome::Continue(model, mut cmd) => {
                assert_eq!(model.api_key_input, "abc123");
                cmd.expect_one_effect().expect_render();
            }
            Outcome::Complete(..) => panic!("Expected Continue"),
        }
    }

    #[test]
    fn test_submit_stores_secret() {
        let model = OnboardModel {
            api_key_input: "my_new_key".to_string(),
            error: None,
        };
        let outcome = model.update(OnboardEvent::Submit);

        match outcome {
            Outcome::Continue(_, mut cmd) => {
                let request = cmd.expect_one_effect().expect_secret();
                assert_eq!(
                    request.operation,
                    SecretRequest::Store(
                        secret::API_KEY_NAME.to_string(),
                        "my_new_key".to_string(),
                    )
                );
            }
            Outcome::Complete(..) => panic!("Expected Continue"),
        }
    }

    #[test]
    fn test_submit_empty_shows_error() {
        let model = OnboardModel {
            api_key_input: "  ".to_string(),
            error: None,
        };
        let outcome = model.update(OnboardEvent::Submit);

        match outcome {
            Outcome::Continue(model, mut cmd) => {
                cmd.expect_one_effect().expect_render();
                assert_eq!(model.error.as_deref(), Some("API key cannot be empty"));
            }
            Outcome::Complete(..) => panic!("Expected Continue"),
        }
    }

    #[test]
    fn test_store_error_shows_message() {
        let model = OnboardModel::default();
        let outcome = model.update(OnboardEvent::SecretStored(
            SecretStoreResponse::StoreError("disk full".to_string()),
        ));

        match outcome {
            Outcome::Continue(model, mut cmd) => {
                cmd.expect_one_effect().expect_render();
                assert!(model.error.as_ref().unwrap().contains("disk full"));
            }
            Outcome::Complete(..) => panic!("Expected Continue"),
        }
    }

    #[test]
    fn test_stored_then_fetched_completes_with_api_key() {
        let model = OnboardModel::default();

        // First: SecretStored -> should re-fetch
        let outcome = model.update(OnboardEvent::SecretStored(
            SecretStoreResponse::Stored(secret::API_KEY_NAME.to_string()),
        ));
        let model = match outcome {
            Outcome::Continue(model, mut cmd) => {
                let request = cmd.expect_one_effect().expect_secret();
                assert_eq!(
                    request.operation,
                    SecretRequest::Fetch(secret::API_KEY_NAME.to_string())
                );
                model
            }
            Outcome::Complete(..) => panic!("Expected Continue after store"),
        };

        // Then: SecretFetched -> should complete
        let outcome = model.update(OnboardEvent::SecretFetched(
            SecretFetchResponse::Fetched("the_key".to_string()),
        ));
        match outcome {
            Outcome::Complete(api_key, _) => {
                assert_eq!(api_key, "the_key");
            }
            Outcome::Continue(..) => panic!("Expected Complete"),
        }
    }
}
