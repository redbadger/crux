use std::time::Duration;

use crux_core::{Command, render::render};
use crux_time::command::{Time, TimerHandle, TimerOutcome};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::http::location::{self as location_api, GeocodingResponse, LocationError};
use crate::model::ApiKey;
use crate::model::outcome::Outcome;
use crate::model::versioned_input::VersionedInput;

use super::model::Favorite;

const DEBOUNCE_MILLIS: u64 = 300;

#[derive(Debug)]
pub struct AddFavoriteWorkflow {
    pub input: VersionedInput,
    pub search_results: Option<Vec<GeocodingResponse>>,
    pub searching: bool,
    timer_handle: Option<TimerHandle>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum AddFavoriteEvent {
    Search(String),
    Submit(Box<GeocodingResponse>),
    Cancel,

    #[serde(skip)]
    #[facet(skip)]
    DebounceComplete(#[facet(opaque)] TimerOutcome),

    #[serde(skip)]
    #[facet(skip)]
    SearchResult(
        usize,
        #[facet(opaque)] Box<Result<Vec<GeocodingResponse>, LocationError>>,
    ),
}

#[derive(Debug)]
pub(crate) enum AddFavoriteTransition {
    Selected(Box<Favorite>),
    Cancelled,
}

impl Default for AddFavoriteWorkflow {
    fn default() -> Self {
        Self::new()
    }
}

impl AddFavoriteWorkflow {
    #[must_use]
    pub fn new() -> Self {
        Self {
            input: VersionedInput::default(),
            search_results: None,
            searching: false,
            timer_handle: None,
        }
    }

    pub(crate) fn update(
        mut self,
        event: AddFavoriteEvent,
        api_key: &ApiKey,
    ) -> Outcome<Self, AddFavoriteTransition, AddFavoriteEvent> {
        match event {
            AddFavoriteEvent::Search(query) => {
                tracing::debug!("updating search input to '{query}'");
                let _version = self.input.update(query);
                self.searching = true;

                if let Some(handle) = self.timer_handle.take() {
                    handle.clear();
                }

                let (notify, handle) = Time::notify_after(Duration::from_millis(DEBOUNCE_MILLIS));
                self.timer_handle = Some(handle);

                let cmd = notify.then_send(AddFavoriteEvent::DebounceComplete);
                Outcome::continuing(self, cmd)
            }

            AddFavoriteEvent::DebounceComplete(TimerOutcome::Completed(_)) => {
                let query = self.input.value().to_string();
                let version = self.input.version();
                tracing::debug!("debounce complete, searching for '{query}' (version {version})");

                let cmd = location_api::fetch(&query, api_key.clone()).then_send(move |result| {
                    AddFavoriteEvent::SearchResult(version, Box::new(result))
                });
                Outcome::continuing(self, cmd)
            }

            AddFavoriteEvent::DebounceComplete(TimerOutcome::Cleared) => {
                tracing::debug!("debounce timer cleared");
                Outcome::continuing(self, Command::done())
            }

            AddFavoriteEvent::SearchResult(version, result) => {
                if !self.input.is_current(version) {
                    tracing::debug!(
                        "ignoring stale search result (version {version}, current {})",
                        self.input.version()
                    );
                    return Outcome::continuing(self, Command::done());
                }

                self.searching = false;
                let results = match *result {
                    Ok(results) => {
                        tracing::debug!("received {} search results", results.len());
                        results
                    }
                    Err(ref err) => {
                        tracing::debug!("search failed: {err:?}");
                        Vec::new()
                    }
                };
                self.search_results = Some(results);
                Outcome::continuing(self, render())
            }

            AddFavoriteEvent::Submit(geo) => {
                tracing::debug!("selecting favorite '{}'", geo.name);
                let favorite = Favorite::from(*geo);
                Outcome::complete(
                    AddFavoriteTransition::Selected(Box::new(favorite)),
                    Command::done(),
                )
            }

            AddFavoriteEvent::Cancel => {
                tracing::debug!("cancelling search");
                Outcome::complete(AddFavoriteTransition::Cancelled, Command::done())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::Effect;

    const TEST_API_KEY: &str = "test_api_key";

    fn test_api_key() -> crate::model::ApiKey {
        TEST_API_KEY.to_string().into()
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

    #[test]
    fn search_updates_input_and_starts_timer() {
        let workflow = AddFavoriteWorkflow::new();
        let api_key = test_api_key();

        let (workflow, mut cmd) = workflow
            .update(AddFavoriteEvent::Search("Phoenix".to_string()), &api_key)
            .expect_continue()
            .into_parts();

        assert_eq!(workflow.input.value(), "Phoenix");
        assert!(workflow.timer_handle.is_some());

        // should have a time effect (the debounce timer)
        let effect = cmd.effects().next().unwrap();
        assert!(matches!(effect, Effect::Time(_)));
    }

    #[test]
    fn second_search_resets_timer() {
        let workflow = AddFavoriteWorkflow::new();
        let api_key = test_api_key();

        let (workflow, _cmd) = workflow
            .update(AddFavoriteEvent::Search("Ph".to_string()), &api_key)
            .expect_continue()
            .into_parts();

        assert_eq!(workflow.input.version(), 1);

        let (workflow, _cmd) = workflow
            .update(AddFavoriteEvent::Search("Pho".to_string()), &api_key)
            .expect_continue()
            .into_parts();

        assert_eq!(workflow.input.value(), "Pho");
        assert_eq!(workflow.input.version(), 2);
    }

    #[test]
    fn stale_search_result_ignored() {
        let mut workflow = AddFavoriteWorkflow::new();
        // simulate: searched at version 1, then typed more (now version 2)
        workflow.input.update("Ph".to_string());
        workflow.input.update("Pho".to_string());
        let api_key = test_api_key();

        let results = vec![test_geocoding()];
        let (workflow, _cmd) = workflow
            .update(
                AddFavoriteEvent::SearchResult(1, Box::new(Ok(results))),
                &api_key,
            )
            .expect_continue()
            .into_parts();

        // stale result should be ignored
        assert!(workflow.search_results.is_none());
    }

    #[test]
    fn current_search_result_applied() {
        let mut workflow = AddFavoriteWorkflow::new();
        workflow.input.update("Phoenix".to_string());
        let api_key = test_api_key();

        let results = vec![test_geocoding()];
        let (workflow, _cmd) = workflow
            .update(
                AddFavoriteEvent::SearchResult(1, Box::new(Ok(results.clone()))),
                &api_key,
            )
            .expect_continue()
            .into_parts();

        assert_eq!(workflow.search_results, Some(results));
    }

    #[test]
    fn submit_completes_with_favorite() {
        let workflow = AddFavoriteWorkflow::new();
        let api_key = test_api_key();
        let geo = test_geocoding();

        let (transition, _cmd) = workflow
            .update(AddFavoriteEvent::Submit(Box::new(geo.clone())), &api_key)
            .expect_complete()
            .into_parts();

        assert!(matches!(transition, AddFavoriteTransition::Selected(fav) if fav.0 == geo));
    }

    #[test]
    fn cancel_completes() {
        let workflow = AddFavoriteWorkflow::new();
        let api_key = test_api_key();

        let (transition, _cmd) = workflow
            .update(AddFavoriteEvent::Cancel, &api_key)
            .expect_complete()
            .into_parts();

        assert!(matches!(transition, AddFavoriteTransition::Cancelled));
    }

    #[test]
    fn search_sets_searching_flag() {
        let workflow = AddFavoriteWorkflow::new();
        let api_key = test_api_key();

        assert!(!workflow.searching);

        let (workflow, _cmd) = workflow
            .update(AddFavoriteEvent::Search("Ph".to_string()), &api_key)
            .expect_continue()
            .into_parts();

        assert!(workflow.searching);
    }

    #[test]
    fn search_result_clears_searching_flag() {
        let mut workflow = AddFavoriteWorkflow::new();
        workflow.input.update("Phoenix".to_string());
        workflow.searching = true;
        let api_key = test_api_key();

        let (workflow, _cmd) = workflow
            .update(
                AddFavoriteEvent::SearchResult(1, Box::new(Ok(vec![test_geocoding()]))),
                &api_key,
            )
            .expect_continue()
            .into_parts();

        assert!(!workflow.searching);
    }

    #[test]
    fn search_error_clears_searching_flag() {
        let mut workflow = AddFavoriteWorkflow::new();
        workflow.input.update("Phoenix".to_string());
        workflow.searching = true;
        let api_key = test_api_key();

        let (workflow, _cmd) = workflow
            .update(
                AddFavoriteEvent::SearchResult(
                    1,
                    Box::new(Err(
                        crate::effects::http::location::LocationError::NetworkError,
                    )),
                ),
                &api_key,
            )
            .expect_continue()
            .into_parts();

        assert!(!workflow.searching);
        assert_eq!(workflow.search_results, Some(vec![]));
    }

    #[test]
    fn debounce_cleared_is_noop() {
        let mut workflow = AddFavoriteWorkflow::new();
        workflow.input.update("Ph".to_string());
        workflow.searching = true;
        let api_key = test_api_key();

        let (workflow, mut cmd) = workflow
            .update(
                AddFavoriteEvent::DebounceComplete(TimerOutcome::Cleared),
                &api_key,
            )
            .expect_continue()
            .into_parts();

        // should produce no effects
        assert!(cmd.effects().next().is_none());
        // searching state unchanged — still waiting for the new timer
        assert!(workflow.searching);
    }
}
