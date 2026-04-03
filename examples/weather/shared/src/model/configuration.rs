use crux_core::{Command, render::render};

use crate::effects::secret::{self, SecretFetchResponse, SecretStoreResponse};

use super::{ConfigurationEvent, outcome::Outcome};

/// Model for the configuration screen where the user enters their API key.
#[derive(Debug)]
pub struct ConfigurationModel {
    pub api_key_input: String,
    pub error: Option<String>,
}

impl Default for ConfigurationModel {
    fn default() -> Self {
        Self {
            api_key_input: String::new(),
            error: None,
        }
    }
}

impl ConfigurationModel {
    /// Process a configuration event. Returns `Complete(api_key)` when the key
    /// has been stored and confirmed, or `Continue(self)` to stay on this screen.
    pub(crate) fn update(
        mut self,
        event: ConfigurationEvent,
    ) -> Outcome<Self, String, ConfigurationEvent> {
        match event {
            ConfigurationEvent::Set(text) => {
                self.api_key_input = text;
                Outcome::Continue(self, render())
            }
            ConfigurationEvent::Submit => {
                let key = self.api_key_input.trim().to_string();
                if key.is_empty() {
                    self.error = Some("API key cannot be empty".to_string());
                    return Outcome::Continue(self, render());
                }
                let cmd = secret::command::store(secret::API_KEY_NAME, key)
                    .then_send(ConfigurationEvent::SecretStored);
                Outcome::Continue(self, cmd)
            }
            ConfigurationEvent::SecretStored(response) => match response {
                SecretStoreResponse::Stored(_) => {
                    let cmd = secret::command::fetch(secret::API_KEY_NAME)
                        .then_send(ConfigurationEvent::SecretFetched);
                    Outcome::Continue(self, cmd)
                }
                SecretStoreResponse::StoreError(msg) => {
                    self.error = Some(format!("Failed to store: {msg}"));
                    Outcome::Continue(self, render())
                }
            },
            ConfigurationEvent::SecretFetched(response) => match response {
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
        let model = ConfigurationModel::default();
        let outcome = model.update(ConfigurationEvent::Set("abc123".to_string()));

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
        let model = ConfigurationModel {
            api_key_input: "my_new_key".to_string(),
            error: None,
        };
        let outcome = model.update(ConfigurationEvent::Submit);

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
        let model = ConfigurationModel {
            api_key_input: "  ".to_string(),
            error: None,
        };
        let outcome = model.update(ConfigurationEvent::Submit);

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
        let model = ConfigurationModel::default();
        let outcome = model.update(ConfigurationEvent::SecretStored(
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
        let model = ConfigurationModel::default();

        // First: SecretStored -> should re-fetch
        let outcome = model.update(ConfigurationEvent::SecretStored(
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
        let outcome = model.update(ConfigurationEvent::SecretFetched(
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
