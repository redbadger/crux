pub mod favorites;
pub mod location;
pub mod weather;

use crux_core::{Command, render::render};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::secret::{self, SecretDeleteResponse};

use super::{ActiveModel, Workflow, outcome::Outcome};
use self::{
    favorites::events::FavoritesEvent,
    weather::events::WeatherEvent,
};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum ActiveEvent {
    Navigate(Box<Workflow>),
    Home(Box<WeatherEvent>),
    Favorites(Box<FavoritesEvent>),
    ResetApiKey,

    #[serde(skip)]
    #[facet(skip)]
    SecretDeleted(#[facet(opaque)] SecretDeleteResponse),
}

/// Transition value when the active state completes.
#[derive(Debug)]
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
                Outcome::continuing(self, cmd)
            }
            ActiveEvent::SecretDeleted(response) => match response {
                SecretDeleteResponse::Deleted(_) => {
                    Outcome::complete(ActiveTransition::ResetApiKey, render())
                }
                SecretDeleteResponse::DeleteError(_) => Outcome::continuing(self, Command::done()),
            },
            ActiveEvent::Navigate(next) => {
                self.workflow = *next;
                Outcome::continuing(self, render())
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

                Outcome::continuing(self, Command::all(commands))
            }
            ActiveEvent::Favorites(fav_event) => {
                let cmd = favorites::events::update(*fav_event, &mut self)
                    .map_event(|e| ActiveEvent::Favorites(Box::new(e)));
                Outcome::continuing(self, cmd)
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

        let mut cmd = outcome.expect_continue().into_command();
        let request = cmd.expect_one_effect().expect_secret();
        assert_eq!(
            request.operation,
            SecretRequest::Delete(secret::API_KEY_NAME.to_string())
        );
    }

    #[test]
    fn test_secret_deleted_completes_with_reset() {
        let model = active_model();
        let outcome = model.update(ActiveEvent::SecretDeleted(SecretDeleteResponse::Deleted(
            secret::API_KEY_NAME.to_string(),
        )));

        let (transition, mut cmd) = outcome.expect_complete().into_parts();
        assert!(matches!(transition, ActiveTransition::ResetApiKey));
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn test_navigation() {
        let model = active_model();
        let outcome = model.update(ActiveEvent::Navigate(Box::new(Workflow::Favorites(
            FavoritesState::Idle,
        ))));

        let (model, mut cmd) = outcome.expect_continue().into_parts();
        assert!(matches!(
            model.workflow,
            Workflow::Favorites(FavoritesState::Idle)
        ));
        cmd.expect_one_effect().expect_render();
    }
}
