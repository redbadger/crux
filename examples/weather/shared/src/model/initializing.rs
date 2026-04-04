use crux_core::{Command, render::render};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::{Effect, secret::{self, SecretFetchResponse}};

use super::{Event, outcome::Outcome};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum InitializingEvent {
    #[serde(skip)]
    #[facet(skip)]
    SecretFetched(#[facet(opaque)] SecretFetchResponse),
}

pub(crate) enum InitializingTransition {
    Onboard,
    Active(String),
}

#[derive(Debug)]
pub struct InitializingModel;

impl InitializingModel {
    pub fn start() -> (Self, Command<Effect, Event>) {
        let cmd = secret::command::fetch(secret::API_KEY_NAME)
            .then_send(|r| Event::Initializing(InitializingEvent::SecretFetched(r)));
        (Self, cmd)
    }

    pub(crate) fn update(
        self,
        event: InitializingEvent,
    ) -> Outcome<Self, InitializingTransition, InitializingEvent> {
        match event {
            InitializingEvent::SecretFetched(response) => match response {
                SecretFetchResponse::Fetched(api_key) => {
                    Outcome::Complete(InitializingTransition::Active(api_key), Command::done())
                }
                SecretFetchResponse::Missing(_) => {
                    Outcome::Complete(InitializingTransition::Onboard, render())
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::effects::secret;

    use super::*;

    #[test]
    fn test_secret_missing_transitions_to_settings() {
        let model = InitializingModel;
        let outcome = model.update(InitializingEvent::SecretFetched(
            SecretFetchResponse::Missing(secret::API_KEY_NAME.to_string()),
        ));

        match outcome {
            Outcome::Complete(InitializingTransition::Onboard, mut cmd) => {
                cmd.expect_one_effect().expect_render();
            }
            _ => panic!("Expected Complete(Onboard)"),
        }
    }

    #[test]
    fn test_secret_fetched_transitions_to_active() {
        let model = InitializingModel;
        let outcome = model.update(InitializingEvent::SecretFetched(
            SecretFetchResponse::Fetched("my_key".to_string()),
        ));

        match outcome {
            Outcome::Complete(InitializingTransition::Active(api_key), _) => {
                assert_eq!(api_key, "my_key");
            }
            _ => panic!("Expected Complete(Active)"),
        }
    }
}
