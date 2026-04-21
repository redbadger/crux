//! Sets the user up to use the application for the first time, by entering their API key.
//!
//! Triggered by: missing api key in secrets during `InitializingModel`
//! Outcome: the `ApiKey` defined and submitted by the user.

use crux_core::render::render;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::secret::{self, SecretStoreResponse};

use super::{
    ApiKey,
    active::favorites::model::Favorites,
    outcome::{Outcome, Started},
};

/// Why the user is on the onboarding screen — lets the shell adjust copy.
#[derive(Facet, Serialize, Deserialize, Copy, Clone, Default, Debug, PartialEq)]
#[repr(C)]
pub enum OnboardReason {
    /// First run — no key has ever been stored.
    #[default]
    Welcome,
    /// The stored key was rejected by the weather API (401).
    Unauthorized,
    /// The user explicitly reset their key from the active app.
    Reset,
}

/// The transition from onboarding when it completes.
#[derive(Debug)]
pub(crate) enum OnboardTransition {
    /// The user successfully set up their API key.
    Active(ApiKey, Favorites),
    /// A system error prevented onboarding from completing.
    Failed(String),
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum OnboardEvent {
    /// The user updates the API key input.
    ApiKey(String),
    /// The user wants to submit their API key that they entered.
    Submit,

    /// The user's API key was successfully stored, and can be retrieved next time.
    #[serde(skip)]
    #[facet(skip)]
    SecretStored(#[facet(opaque)] SecretStoreResponse),
}

/// Model for the onboarding screen where the user enters their API key.
#[derive(Debug)]
pub struct OnboardModel {
    pub reason: OnboardReason,
    pub state: OnboardState,
    favorites: Favorites,
}

#[derive(Debug)]
pub enum OnboardState {
    /// The user is entering their API key.
    Input { api_key: String },
    /// The API key is being saved to the secret store.
    Saving { api_key: ApiKey },
}

impl Default for OnboardModel {
    fn default() -> Self {
        Self::new(OnboardReason::default(), Favorites::default())
    }
}

impl OnboardModel {
    #[must_use]
    pub fn new(reason: OnboardReason, favorites: Favorites) -> Self {
        Self {
            reason,
            favorites,
            state: OnboardState::Input {
                api_key: String::new(),
            },
        }
    }

    pub(crate) fn start(
        reason: OnboardReason,
        favorites: Favorites,
    ) -> Started<Self, OnboardEvent> {
        tracing::debug!("starting onboarding");
        Started::new(Self::new(reason, favorites), render())
    }
}

impl OnboardModel {
    /// Whether the user can submit the API key (non-empty and in the input state).
    #[must_use]
    pub fn can_submit(&self) -> bool {
        matches!(self.state, OnboardState::Input { ref api_key } if !api_key.trim().is_empty())
    }

    /// Creates a failure completion of this model with the given message.
    fn fail(msg: String) -> Outcome<Self, OnboardTransition, OnboardEvent> {
        tracing::error!("{msg}");
        Outcome::complete(OnboardTransition::Failed(msg), render())
    }

    /// Process an onboard event. Returns `Complete` with either an `Active` transition
    /// (API key stored) or a `Failed` transition (system error), or `Continue(self)`
    /// to stay on this screen.
    pub(crate) fn update(
        self,
        event: OnboardEvent,
    ) -> Outcome<Self, OnboardTransition, OnboardEvent> {
        let Self {
            reason,
            state,
            favorites,
        } = self;

        match (state, event) {
            (OnboardState::Input { .. }, OnboardEvent::ApiKey(text)) => {
                tracing::debug!("updating api key input, waiting for submission");
                Outcome::continuing(
                    Self {
                        reason,
                        favorites,
                        state: OnboardState::Input { api_key: text },
                    },
                    render(),
                )
            }
            (OnboardState::Input { ref api_key }, OnboardEvent::Submit)
                if api_key.trim().is_empty() =>
            {
                Self::fail("expecting a non-empty API key before submitting".to_string())
            }
            (OnboardState::Input { api_key }, OnboardEvent::Submit) => {
                tracing::debug!("submitting api key, saving to secret store");
                let key = api_key.trim().to_string();
                let cmd = secret::command::store(secret::API_KEY_NAME, key.clone())
                    .then_send(OnboardEvent::SecretStored);
                Outcome::continuing(
                    Self {
                        reason,
                        favorites,
                        state: OnboardState::Saving {
                            api_key: key.into(),
                        },
                    },
                    cmd,
                )
            }
            (OnboardState::Saving { api_key }, OnboardEvent::SecretStored(response)) => {
                match response {
                    SecretStoreResponse::Stored(_) => {
                        tracing::debug!("completing onboarding, api key stored successfully");
                        Outcome::complete(OnboardTransition::Active(api_key, favorites), render())
                    }
                    SecretStoreResponse::StoreError(msg) => Self::fail(format!(
                        "failed to store API key in the secret store due to error: {msg}"
                    )),
                }
            }
            (state, event) => Self::fail(format!(
                "cannot update with event given the current state of the model, got {event:?} in {state:?}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::effects::secret::{self, SecretRequest};
    use rstest::rstest;

    use super::*;

    fn input_model(api_key: &str) -> OnboardModel {
        OnboardModel {
            reason: OnboardReason::default(),
            favorites: Favorites::default(),
            state: OnboardState::Input {
                api_key: api_key.to_string(),
            },
        }
    }

    fn saving_model(api_key: &str) -> OnboardModel {
        OnboardModel {
            reason: OnboardReason::default(),
            favorites: Favorites::default(),
            state: OnboardState::Saving {
                api_key: api_key.to_string().into(),
            },
        }
    }

    #[rstest]
    #[case::empty_cannot_submit("", false)]
    #[case::whitespace_cannot_submit("  ", false)]
    #[case::valid_key_can_submit("abc123", true)]
    fn can_submit(#[case] input: &str, #[case] expected: bool) {
        let model = input_model(input);
        assert_eq!(model.can_submit(), expected);
    }

    #[test]
    fn api_key_updates_input() {
        let model = OnboardModel::default();
        let outcome = model.update(OnboardEvent::ApiKey("abc123".to_string()));

        let (model, mut cmd) = outcome.expect_continue().into_parts();
        assert!(matches!(
            model.state,
            OnboardState::Input { ref api_key } if api_key == "abc123"
        ));
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn submit_stores_secret() {
        let model = input_model("my_new_key");
        let outcome = model.update(OnboardEvent::Submit);

        let (model, mut cmd) = outcome.expect_continue().into_parts();
        assert!(
            matches!(model.state, OnboardState::Saving { ref api_key } if *api_key == "my_new_key")
        );
        let request = cmd.expect_one_effect().expect_secret();
        assert_eq!(
            request.operation,
            SecretRequest::Store(secret::API_KEY_NAME.to_string(), "my_new_key".to_string(),)
        );
    }

    #[test]
    fn submit_empty_fails() {
        let model = input_model("  ");
        assert!(!model.can_submit());
        let outcome = model.update(OnboardEvent::Submit);

        let (transition, mut cmd) = outcome.expect_complete().into_parts();
        cmd.expect_one_effect().expect_render();
        assert_let_bind::assert_let!(OnboardTransition::Failed(msg), transition);
        assert!(msg.contains("empty"));
    }

    #[test]
    fn unexpected_event_fails() {
        let model = saving_model("key");
        let outcome = model.update(OnboardEvent::ApiKey("oops".to_string()));

        let (transition, mut cmd) = outcome.expect_complete().into_parts();
        cmd.expect_one_effect().expect_render();
        assert_let_bind::assert_let!(OnboardTransition::Failed(_), transition);
    }

    #[test]
    fn store_error_fails_with_message() {
        let model = saving_model("some_key");
        let outcome = model.update(OnboardEvent::SecretStored(SecretStoreResponse::StoreError(
            "disk full".to_string(),
        )));

        let (transition, mut cmd) = outcome.expect_complete().into_parts();
        cmd.expect_one_effect().expect_render();
        assert_let_bind::assert_let!(OnboardTransition::Failed(msg), transition);
        assert!(msg.contains("disk full"));
    }

    #[test]
    fn stored_completes_with_api_key() {
        let model = saving_model("the_key");
        let outcome = model.update(OnboardEvent::SecretStored(SecretStoreResponse::Stored(
            secret::API_KEY_NAME.to_string(),
        )));

        let (transition, mut cmd) = outcome.expect_complete().into_parts();
        cmd.expect_one_effect().expect_render();
        assert_let_bind::assert_let!(OnboardTransition::Active(api_key, _favorites), transition);
        assert_eq!(api_key, "the_key");
    }
}
