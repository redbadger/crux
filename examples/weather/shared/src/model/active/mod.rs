pub mod favorites;
pub mod home;

use crux_core::{Command, render::render};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::secret::{self, SecretDeleteResponse};

use super::{ApiKey, outcome::Outcome};
use self::{
    favorites::{FavoritesScreen, FavoritesScreenEvent, FavoritesTransition, model::Favorites},
    home::{HomeEvent, HomeScreen, HomeTransition},
};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum ActiveEvent {
    Home(Box<HomeEvent>),
    Favorites(Box<FavoritesScreenEvent>),
    ResetApiKey,

    #[serde(skip)]
    #[facet(skip)]
    SecretDeleted(#[facet(opaque)] SecretDeleteResponse),
}

impl ActiveEvent {
    pub fn home(event: HomeEvent) -> Self {
        ActiveEvent::Home(Box::new(event))
    }

    pub fn favorites(event: FavoritesScreenEvent) -> Self {
        ActiveEvent::Favorites(Box::new(event))
    }
}

#[derive(Debug)]
pub enum Screen {
    Home(HomeScreen),
    Favorites(FavoritesScreen),
}

impl Default for Screen {
    fn default() -> Self {
        Screen::Home(HomeScreen::default())
    }
}

impl Screen {
    fn into_favorites(self) -> Favorites {
        match self {
            Screen::Home(home) => home.favorites_weather.into(),
            Screen::Favorites(fav) => fav.favorites,
        }
    }
}

/// Transition value when the active state completes.
#[derive(Debug)]
pub(crate) enum ActiveTransition {
    ResetApiKey(Favorites),
    Unauthorized(Favorites),
}

#[derive(Default, Debug)]
pub struct ActiveModel {
    pub api_key: ApiKey,
    pub screen: Screen,
}

impl ActiveModel {
    pub(crate) fn update(
        self,
        event: ActiveEvent,
    ) -> Outcome<Self, ActiveTransition, ActiveEvent> {
        let ActiveModel { api_key, screen } = self;

        match event {
            ActiveEvent::ResetApiKey => {
                let cmd = secret::command::delete(secret::API_KEY_NAME)
                    .then_send(ActiveEvent::SecretDeleted);
                Outcome::continuing(ActiveModel { api_key, screen }, cmd)
            }
            ActiveEvent::SecretDeleted(response) => match response {
                SecretDeleteResponse::Deleted(_) => {
                    Outcome::complete(ActiveTransition::ResetApiKey(screen.into_favorites()), render())
                }
                SecretDeleteResponse::DeleteError(_) => {
                    Outcome::continuing(ActiveModel { api_key, screen }, Command::done())
                }
            },
            ActiveEvent::Home(home_event) => {
                let Screen::Home(home) = screen else {
                    return Outcome::continuing(ActiveModel { api_key, screen }, Command::done());
                };

                let (status, cmd) = home
                    .update(*home_event, &api_key)
                    .map_event(ActiveEvent::home)
                    .into_parts();

                match status {
                    super::outcome::Status::Continue(home) => Outcome::continuing(
                        ActiveModel {
                            api_key,
                            screen: Screen::Home(home),
                        },
                        cmd,
                    ),
                    super::outcome::Status::Complete(HomeTransition::GoToFavorites(favorites)) => {
                        Outcome::continuing(
                            ActiveModel {
                                api_key,
                                screen: Screen::Favorites(FavoritesScreen {
                                    favorites,
                                    workflow: None,
                                }),
                            },
                            cmd,
                        )
                    }
                    super::outcome::Status::Complete(HomeTransition::ApiKeyRejected(favorites)) => {
                        Outcome::complete(ActiveTransition::Unauthorized(favorites), cmd.and(render()))
                    }
                }
            }
            ActiveEvent::Favorites(fav_event) => {
                let Screen::Favorites(fav_screen) = screen else {
                    return Outcome::continuing(ActiveModel { api_key, screen }, Command::done());
                };

                let (status, cmd) = fav_screen
                    .update(*fav_event, &api_key)
                    .map_event(ActiveEvent::favorites)
                    .into_parts();

                match status {
                    super::outcome::Status::Continue(fav_screen) => Outcome::continuing(
                        ActiveModel {
                            api_key,
                            screen: Screen::Favorites(fav_screen),
                        },
                        cmd,
                    ),
                    super::outcome::Status::Complete(FavoritesTransition::GoToHome(favorites)) => {
                        let (home_screen, start_cmd) = HomeScreen::start(favorites, &api_key)
                            .map_event(ActiveEvent::home)
                            .into_parts();
                        Outcome::continuing(
                            ActiveModel {
                                api_key,
                                screen: Screen::Home(home_screen),
                            },
                            cmd.and(start_cmd),
                        )
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::effects::secret::{self, SecretRequest};

    use super::*;
    use super::favorites::model::Favorites;

    fn active_model() -> ActiveModel {
        ActiveModel {
            api_key: "test_api_key".to_string().into(),
            ..Default::default()
        }
    }

    #[test]
    fn reset_api_key_deletes_secret() {
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
    fn secret_deleted_completes_with_reset() {
        let model = active_model();
        let outcome = model.update(ActiveEvent::SecretDeleted(SecretDeleteResponse::Deleted(
            secret::API_KEY_NAME.to_string(),
        )));

        let (transition, mut cmd) = outcome.expect_complete().into_parts();
        assert!(matches!(transition, ActiveTransition::ResetApiKey(_)));
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn home_go_to_favorites_transition() {
        let model = active_model();
        let outcome = model.update(ActiveEvent::home(HomeEvent::GoToFavorites));

        let (model, mut cmd) = outcome.expect_continue().into_parts();
        cmd.expect_one_effect().expect_render();
        assert!(matches!(
            model.screen,
            Screen::Favorites(FavoritesScreen {
                workflow: None,
                ..
            })
        ));
    }

    #[test]
    fn favorites_go_to_home_transition() {
        let model = ActiveModel {
            api_key: "test_api_key".to_string().into(),
            screen: Screen::Favorites(FavoritesScreen {
                favorites: Favorites::default(),
                workflow: None,
            }),
        };
        let outcome = model.update(ActiveEvent::favorites(FavoritesScreenEvent::GoToHome));

        let (model, _cmd) = outcome.expect_continue().into_parts();
        assert!(matches!(model.screen, Screen::Home(_)));
    }

    #[test]
    fn favorites_go_to_add_favorite() {
        let model = ActiveModel {
            api_key: "test_api_key".to_string().into(),
            screen: Screen::Favorites(FavoritesScreen {
                favorites: Favorites::default(),
                workflow: None,
            }),
        };
        let outcome =
            model.update(ActiveEvent::favorites(FavoritesScreenEvent::RequestAddFavorite));

        let (model, mut cmd) = outcome.expect_continue().into_parts();
        cmd.expect_one_effect().expect_render();
        let Screen::Favorites(fav) = &model.screen else {
            panic!("Expected Favorites screen");
        };
        assert!(matches!(
            fav.workflow,
            Some(favorites::FavoritesWorkflow::Add(_))
        ));
    }
}
