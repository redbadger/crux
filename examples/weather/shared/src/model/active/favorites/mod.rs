//! Screen for managing the user's favorite locations, where they want to check weather from.

pub mod add;
pub mod confirm_delete;
pub mod model;

use crux_core::{Command, render::render};
use crux_kv::command::KeyValue;
use crux_kv::error::KeyValueError;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::location::Location;
use crate::model::ApiKey;
use crate::model::outcome::{Outcome, Status};

use self::add::{AddFavoriteEvent, AddFavoriteTransition, AddFavoriteWorkflow};
use self::confirm_delete::{ConfirmDeleteEvent, ConfirmDeleteTransition, ConfirmDeleteWorkflow};
use self::model::{FAVORITES_KEY, Favorites};

/// Events for the favorites screen.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum FavoritesScreenEvent {
    GoToHome,
    RequestAddFavorite,
    RequestDelete(Location),
    Workflow(FavoritesWorkflowEvent),

    #[serde(skip)]
    #[facet(skip)]
    Persisted(#[facet(opaque)] Result<Option<Vec<u8>>, KeyValueError>),
}

/// Events dispatched to the active workflow.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum FavoritesWorkflowEvent {
    Add(AddFavoriteEvent),
    ConfirmDelete(ConfirmDeleteEvent),
}

impl FavoritesScreenEvent {
    pub fn add(event: AddFavoriteEvent) -> Self {
        Self::Workflow(FavoritesWorkflowEvent::Add(event))
    }

    pub fn confirm_delete(event: ConfirmDeleteEvent) -> Self {
        Self::Workflow(FavoritesWorkflowEvent::ConfirmDelete(event))
    }
}

/// Screen for managing the favorite locations that the user wants to see weather for.
#[derive(Debug)]
pub struct FavoritesScreen {
    pub favorites: Favorites,
    pub workflow: Option<FavoritesWorkflow>,
}

/// A focused workflow within the favorites screen.
#[derive(Debug)]
pub enum FavoritesWorkflow {
    /// Add a favorite location to the favorites.
    Add(AddFavoriteWorkflow),
    /// Confirm that a favorite should be deleted.
    ConfirmDelete(ConfirmDeleteWorkflow),
}

/// Valid transitions from the favorites screen.
#[derive(Debug)]
pub(crate) enum FavoritesTransition {
    /// Transition to the home screen.
    GoToHome(Favorites),
}

fn persist_favorites(favorites: &Favorites) -> Command<crate::effects::Effect, FavoritesScreenEvent>
{
    KeyValue::set(
        FAVORITES_KEY,
        serde_json::to_vec(favorites.as_slice()).unwrap(),
    )
    .then_send(FavoritesScreenEvent::Persisted)
}

impl FavoritesScreen {
    pub(crate) fn update(
        mut self,
        event: FavoritesScreenEvent,
        api_key: &ApiKey,
    ) -> Outcome<Self, FavoritesTransition, FavoritesScreenEvent> {
        match event {
            FavoritesScreenEvent::GoToHome => {
                tracing::debug!("navigating to home screen");
                Outcome::complete(FavoritesTransition::GoToHome(self.favorites), render())
            }

            FavoritesScreenEvent::RequestAddFavorite => {
                tracing::debug!("starting add favorite workflow");
                self.workflow = Some(FavoritesWorkflow::Add(AddFavoriteWorkflow::new()));
                Outcome::continuing(self, render())
            }

            FavoritesScreenEvent::RequestDelete(location) => {
                tracing::debug!("starting delete confirmation workflow for {location:?}");
                self.workflow = Some(FavoritesWorkflow::ConfirmDelete(
                    ConfirmDeleteWorkflow::new(location),
                ));
                Outcome::continuing(self, render())
            }

            FavoritesScreenEvent::Workflow(workflow_event) => {
                let Some(workflow) = self.workflow.take() else {
                    return Outcome::continuing(self, Command::done());
                };

                match (workflow, workflow_event) {
                    (
                        FavoritesWorkflow::Add(add_workflow),
                        FavoritesWorkflowEvent::Add(event),
                    ) => {
                        let (status, cmd) = add_workflow
                            .update(event, api_key)
                            .map_event(|e| {
                                FavoritesScreenEvent::Workflow(FavoritesWorkflowEvent::Add(e))
                            })
                            .into_parts();

                        match status {
                            Status::Continue(add_workflow) => {
                                self.workflow = Some(FavoritesWorkflow::Add(add_workflow));
                                Outcome::continuing(self, cmd)
                            }
                            Status::Complete(AddFavoriteTransition::Selected(favorite)) => {
                                if self.favorites.exists(&favorite.location()) {
                                    tracing::debug!("ignoring duplicate favorite {:?}", favorite.location());
                                    Outcome::continuing(self, cmd.and(render()))
                                } else {
                                    tracing::debug!("adding favorite '{}' and persisting", favorite.name());
                                    self.favorites.insert(favorite);
                                    let persist = persist_favorites(&self.favorites);
                                    Outcome::continuing(
                                        self,
                                        cmd.and(render()).and(persist),
                                    )
                                }
                            }
                            Status::Complete(AddFavoriteTransition::Cancelled) => {
                                tracing::debug!("cancelling add favorite workflow");
                                Outcome::continuing(self, cmd.and(render()))
                            }
                        }
                    }

                    (
                        FavoritesWorkflow::ConfirmDelete(delete_workflow),
                        FavoritesWorkflowEvent::ConfirmDelete(event),
                    ) => {
                        let (status, cmd) = delete_workflow
                            .update(event)
                            .map_event(|e| {
                                FavoritesScreenEvent::Workflow(
                                    FavoritesWorkflowEvent::ConfirmDelete(e),
                                )
                            })
                            .into_parts();

                        match status {
                            Status::Continue(delete_workflow) => {
                                self.workflow = Some(FavoritesWorkflow::ConfirmDelete(delete_workflow));
                                Outcome::continuing(self, cmd)
                            }
                            Status::Complete(ConfirmDeleteTransition::Confirmed(location)) => {
                                if self.favorites.remove(&location).is_some() {
                                    tracing::debug!("deleting favorite at {location:?} and persisting");
                                    let persist = persist_favorites(&self.favorites);
                                    Outcome::continuing(self, cmd.and(persist))
                                } else {
                                    tracing::debug!("favorite at {location:?} not found, nothing to delete");
                                    Outcome::continuing(self, cmd)
                                }
                            }
                            Status::Complete(ConfirmDeleteTransition::Cancelled) => {
                                tracing::debug!("cancelling delete confirmation workflow");
                                Outcome::continuing(self, cmd)
                            }
                        }
                    }

                    (workflow, event) => {
                        tracing::error!(
                            "workflow event {event:?} does not match active workflow {workflow:?}"
                        );
                        Outcome::continuing(self, render())
                    }
                }
            }

            FavoritesScreenEvent::Persisted(result) => {
                match result {
                    Ok(_) => tracing::debug!("persisted favorites to key-value store"),
                    Err(ref err) => tracing::error!("persisting favorites failed: {err}"),
                }
                Outcome::continuing(self, Command::done())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::http::location::GeocodingResponse;
    use crate::model::active::favorites::model::Favorite;

    const TEST_API_KEY: &str = "test_api_key";

    fn test_api_key() -> crate::model::ApiKey {
        TEST_API_KEY.to_string().into()
    }

    fn test_screen() -> FavoritesScreen {
        FavoritesScreen {
            favorites: Favorites::default(),
            workflow: None,
        }
    }

    fn test_geocoding() -> GeocodingResponse {
        GeocodingResponse {
            name: "Phoenix".to_string(),
            local_names: None,
            lat: 33.456_789,
            lon: -112.037_222,
            country: "US".to_string(),
            state: None,
        }
    }

    fn test_favorite() -> Favorite {
        Favorite::from(test_geocoding())
    }

    fn screen_with_favorite() -> FavoritesScreen {
        let mut screen = test_screen();
        screen.favorites.insert(test_favorite());
        screen
    }

    #[test]
    fn go_to_home_completes_with_favorites() {
        let screen = screen_with_favorite();
        let (transition, mut cmd) = screen
            .update(FavoritesScreenEvent::GoToHome, &test_api_key())
            .expect_complete()
            .into_parts();

        cmd.expect_one_effect().expect_render();
        assert!(matches!(transition, FavoritesTransition::GoToHome(fav) if fav.len() == 1));
    }

    #[test]
    fn request_add_favorite_starts_workflow() {
        let screen = test_screen();
        let (screen, mut cmd) = screen
            .update(FavoritesScreenEvent::RequestAddFavorite, &test_api_key())
            .expect_continue()
            .into_parts();

        cmd.expect_one_effect().expect_render();
        assert!(matches!(screen.workflow, Some(FavoritesWorkflow::Add(_))));
    }

    #[test]
    fn request_delete_starts_workflow() {
        let screen = test_screen();
        let location = test_favorite().location();
        let (screen, mut cmd) = screen
            .update(FavoritesScreenEvent::RequestDelete(location), &test_api_key())
            .expect_continue()
            .into_parts();

        cmd.expect_one_effect().expect_render();
        assert!(matches!(
            screen.workflow,
            Some(FavoritesWorkflow::ConfirmDelete(_))
        ));
    }

    #[test]
    fn submit_adds_favorite_and_persists() {
        let mut screen = test_screen();
        screen.workflow = Some(FavoritesWorkflow::Add(AddFavoriteWorkflow::new()));

        let geo = test_geocoding();
        let (screen, mut cmd) = screen
            .update(
                FavoritesScreenEvent::add(AddFavoriteEvent::Submit(Box::new(geo))),
                &test_api_key(),
            )
            .expect_continue()
            .into_parts();

        assert_eq!(screen.favorites.len(), 1);
        assert!(screen.workflow.is_none());

        // should have a render and a KV set
        let effects: Vec<_> = cmd.effects().collect();
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn submit_duplicate_does_not_persist() {
        let mut screen = screen_with_favorite();
        screen.workflow = Some(FavoritesWorkflow::Add(AddFavoriteWorkflow::new()));

        let geo = test_geocoding();
        let (screen, mut cmd) = screen
            .update(
                FavoritesScreenEvent::add(AddFavoriteEvent::Submit(Box::new(geo))),
                &test_api_key(),
            )
            .expect_continue()
            .into_parts();

        assert_eq!(screen.favorites.len(), 1);
        assert!(screen.workflow.is_none());

        // only a render, no KV set
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn cancel_add_clears_workflow() {
        let mut screen = test_screen();
        screen.workflow = Some(FavoritesWorkflow::Add(AddFavoriteWorkflow::new()));

        let (screen, mut cmd) = screen
            .update(
                FavoritesScreenEvent::add(AddFavoriteEvent::Cancel),
                &test_api_key(),
            )
            .expect_continue()
            .into_parts();

        assert!(screen.workflow.is_none());
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn delete_confirmed_removes_and_persists() {
        let mut screen = screen_with_favorite();
        let location = test_favorite().location();
        screen.workflow = Some(FavoritesWorkflow::ConfirmDelete(
            ConfirmDeleteWorkflow::new(location),
        ));

        let (screen, mut cmd) = screen
            .update(
                FavoritesScreenEvent::confirm_delete(ConfirmDeleteEvent::Confirmed),
                &test_api_key(),
            )
            .expect_continue()
            .into_parts();

        assert_eq!(screen.favorites.len(), 0);
        assert!(screen.workflow.is_none());

        // render from the workflow + KV set from persist
        let effects: Vec<_> = cmd.effects().collect();
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn delete_cancelled_keeps_favorite() {
        let mut screen = screen_with_favorite();
        let location = test_favorite().location();
        screen.workflow = Some(FavoritesWorkflow::ConfirmDelete(
            ConfirmDeleteWorkflow::new(location),
        ));

        let (screen, mut cmd) = screen
            .update(
                FavoritesScreenEvent::confirm_delete(ConfirmDeleteEvent::Cancelled),
                &test_api_key(),
            )
            .expect_continue()
            .into_parts();

        assert_eq!(screen.favorites.len(), 1);
        assert!(screen.workflow.is_none());
        cmd.expect_one_effect().expect_render();
    }
}
