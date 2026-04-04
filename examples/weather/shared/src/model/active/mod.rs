pub mod favorites;
pub mod location;
pub mod weather;

use crux_core::{Command, render::render};

use crate::effects::secret::{self, SecretDeleteResponse};

use super::{ActiveEvent, ActiveModel, outcome::Outcome};
use self::{
    favorites::events::FavoritesEvent,
    weather::events::WeatherEvent,
};

/// Transition value when the active state completes.
pub(crate) enum ActiveTransition {
    ResetApiKey,
}

impl ActiveModel {
    pub(crate) fn update(
        mut self,
        event: ActiveEvent,
    ) -> Outcome<Self, ActiveTransition, ActiveEvent> {
        match event {
            ActiveEvent::ResetApiKey => {
                let cmd = secret::command::delete(secret::API_KEY_NAME)
                    .then_send(ActiveEvent::SecretDeleted);
                Outcome::Continue(self, cmd)
            }
            ActiveEvent::SecretDeleted(response) => match response {
                SecretDeleteResponse::Deleted(_) => {
                    Outcome::Complete(ActiveTransition::ResetApiKey, render())
                }
                SecretDeleteResponse::DeleteError(_) => Outcome::Continue(self, Command::done()),
            },
            ActiveEvent::Navigate(next) => {
                self.workflow = *next;
                Outcome::Continue(self, render())
            }
            ActiveEvent::Home(home_event) => {
                let mut commands = Vec::new();
                if let WeatherEvent::Show = *home_event {
                    commands.push(
                        favorites::events::update(FavoritesEvent::Restore, &mut self)
                            .map_event(|fe| ActiveEvent::Favorites(Box::new(fe))),
                    );
                }

                commands.push(
                    weather::events::update(*home_event, &mut self)
                        .map_event(|we| ActiveEvent::Home(Box::new(we))),
                );

                Outcome::Continue(self, Command::all(commands))
            }
            ActiveEvent::Favorites(fav_event) => {
                let cmd = favorites::events::update(*fav_event, &mut self)
                    .map_event(|e| ActiveEvent::Favorites(Box::new(e)));
                Outcome::Continue(self, cmd)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        effects::secret::{self, SecretRequest},
        model::{ActiveModel, Workflow},
    };
    use super::favorites::model::FavoritesState;

    use super::*;

    fn active_model() -> ActiveModel {
        ActiveModel {
            api_key: "test_api_key".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_reset_api_key_deletes_secret() {
        let model = active_model();
        let outcome = model.update(ActiveEvent::ResetApiKey);

        match outcome {
            Outcome::Continue(_, mut cmd) => {
                let request = cmd.expect_one_effect().expect_secret();
                assert_eq!(
                    request.operation,
                    SecretRequest::Delete(secret::API_KEY_NAME.to_string())
                );
            }
            Outcome::Complete(..) => panic!("Expected Continue (delete is in-flight)"),
        }
    }

    #[test]
    fn test_secret_deleted_completes_with_reset() {
        let model = active_model();
        let outcome = model.update(ActiveEvent::SecretDeleted(SecretDeleteResponse::Deleted(
            secret::API_KEY_NAME.to_string(),
        )));

        match outcome {
            Outcome::Complete(ActiveTransition::ResetApiKey, mut cmd) => {
                cmd.expect_one_effect().expect_render();
            }
            _ => panic!("Expected Complete(ResetApiKey)"),
        }
    }

    #[test]
    fn test_navigation() {
        let model = active_model();
        let outcome = model.update(ActiveEvent::Navigate(Box::new(Workflow::Favorites(
            FavoritesState::Idle,
        ))));

        match outcome {
            Outcome::Continue(model, mut cmd) => {
                assert!(matches!(
                    model.workflow,
                    Workflow::Favorites(FavoritesState::Idle)
                ));
                cmd.expect_one_effect().expect_render();
            }
            Outcome::Complete(..) => panic!("Expected Continue"),
        }
    }
}
