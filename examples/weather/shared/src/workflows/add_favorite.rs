use crux_core::{render::render, Command};
use crux_http::command::Http;
use serde::{Deserialize, Serialize};

use super::favorites::{Favorite, FavoritesEvent, FavoritesState};
use crate::{events::current::API_KEY, Effect, Event, GeocodingResponse, Workflow};

pub const GEOCODING_URL: &str = "https://api.openweathermap.org/geo/1.0/direct";

#[derive(Serialize)]
pub struct GeocodingQueryString {
    pub q: String,
    pub limit: &'static str,
    pub appid: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AddFavoriteEvent {
    Search(String),
    #[serde(skip)]
    SearchResult(Box<crux_http::Result<crux_http::Response<Vec<GeocodingResponse>>>>),
    Submit(Box<GeocodingResponse>),
    Cancel,
}

pub fn update(event: AddFavoriteEvent, model: &mut crate::Model) -> Command<Effect, Event> {
    match event {
        // TODO: use a Time Capability and debounce the search
        // TODO: Search should be a part of events/geocoding.rs
        AddFavoriteEvent::Search(query) => Http::get(GEOCODING_URL)
            .expect_json()
            .query(&GeocodingQueryString {
                q: query,
                limit: "5",
                appid: API_KEY.clone(),
            })
            .expect("could not serialize query string")
            .build()
            .then_send(|result| {
                Event::AddFavorite(Box::new(AddFavoriteEvent::SearchResult(Box::new(result))))
            }),
        AddFavoriteEvent::SearchResult(result) => {
            match *result {
                Ok(mut response) => {
                    let results = response.take_body().unwrap();
                    model.search_results = Some(results);
                }
                Err(_) => {
                    model.search_results = Some(Vec::new());
                }
            }
            render()
        }
        AddFavoriteEvent::Submit(geo) => {
            let favorite = Favorite::from(*geo);
            // Check if a favorite with the same coordinates already exists
            if model.favorites.iter().any(|f| {
                f.geo.lat.to_bits() == favorite.geo.lat.to_bits()
                    && f.geo.lon.to_bits() == favorite.geo.lon.to_bits()
            }) {
                // If it's a duplicate, just return to favorites view without adding
                // TODO: show a toast message
                model.search_results = None;
                model.page = Workflow::Favorites(FavoritesState::Idle);
                render()
            } else {
                model.favorites.push(favorite.clone());
                model.search_results = None;
                model.page = Workflow::Favorites(FavoritesState::Idle);
                render().and(Command::event(Event::Favorites(Box::new(
                    FavoritesEvent::Set,
                ))))
            }
        }
        AddFavoriteEvent::Cancel => {
            model.search_results = None;
            model.page = Workflow::Favorites(FavoritesState::Idle);
            render()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        App, Effect, Event, GeocodingResponse, Model, SAMPLE_GEOCODING_RESPONSE,
        SAMPLE_GEOCODING_RESPONSE_JSON,
    };
    use crux_core::{assert_effect, App as _};
    use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};

    // Helper to create a test geocoding response
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
    fn test_submit_adds_favorite() {
        let mut model = crate::Model::default();
        let geo = test_geocoding();

        // Submit the favorite
        let mut cmd = update(AddFavoriteEvent::Submit(Box::new(geo.clone())), &mut model);

        // Verify we get the Set event
        let event = cmd.events().next().unwrap();
        if let Event::Favorites(event) = &event {
            assert!(matches!(**event, FavoritesEvent::Set));
        } else {
            panic!("Expected Favorites event")
        }

        // Verify the favorite was added and state was updated
        assert_eq!(model.favorites.len(), 1);
        assert_eq!(model.favorites[0].geo, geo);
        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::Idle)
        ));
    }

    #[test]
    fn test_cancel_returns_to_favorites() {
        let mut model = Model {
            page: Workflow::AddFavorite,
            ..Default::default()
        };

        let mut cmd = update(AddFavoriteEvent::Cancel, &mut model);

        // Verify we get a render effect
        assert!(matches!(cmd.effects().next(), Some(Effect::Render(_))));

        // Verify the state was reset to Favorites
        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::Idle)
        ));

        // Verify no favorites were added
        assert!(model.favorites.is_empty());
    }

    #[test]
    fn test_submit_persists_favorite() {
        let mut model = crate::Model::default();
        let geo1 = test_geocoding();
        let geo2 = GeocodingResponse {
            name: "New York".to_string(),
            local_names: None,
            lat: 40.7128,
            lon: -74.0060,
            country: "US".to_string(),
            state: None,
        };

        // Submit first favorite
        let mut cmd = update(AddFavoriteEvent::Submit(Box::new(geo1.clone())), &mut model);
        let event = cmd.events().next().unwrap();
        if let Event::Favorites(event) = &event {
            assert!(matches!(**event, FavoritesEvent::Set));
        } else {
            panic!("Expected Favorites event")
        }
        assert_eq!(model.favorites.len(), 1);
        assert_eq!(model.favorites[0].geo, geo1);

        // Submit second favorite (different location)
        let mut cmd = update(AddFavoriteEvent::Submit(Box::new(geo2.clone())), &mut model);
        let event = cmd.events().next().unwrap();
        if let Event::Favorites(event) = &event {
            assert!(matches!(**event, FavoritesEvent::Set));
        } else {
            panic!("Expected Favorites event")
        }

        // Verify both favorites are in the list
        assert_eq!(model.favorites.len(), 2);
        assert_eq!(model.favorites[0].geo, geo1);
        assert_eq!(model.favorites[1].geo, geo2);

        // Verify we can't add the same favorite again
        let mut cmd = update(AddFavoriteEvent::Submit(Box::new(geo1.clone())), &mut model);
        assert!(cmd.events().next().is_none()); // No Set event for duplicate
        assert_eq!(model.favorites.len(), 2); // List unchanged
    }

    #[test]
    fn test_add_multiple_favorites() {
        let mut model = crate::Model::default();
        let geo1 = test_geocoding();
        let geo2 = GeocodingResponse {
            name: "New York".to_string(),
            local_names: None,
            lat: 40.7128,
            lon: -74.0060,
            country: "US".to_string(),
            state: None,
        };

        // Add first favorite
        let mut cmd = update(AddFavoriteEvent::Submit(Box::new(geo1.clone())), &mut model);
        let event = cmd.events().next().unwrap();
        if let Event::Favorites(event) = &event {
            assert!(matches!(**event, FavoritesEvent::Set));
        } else {
            panic!("Expected Favorites event")
        }
        assert_eq!(model.favorites.len(), 1);
        assert_eq!(model.favorites[0].geo, geo1);

        // Add second favorite
        let mut cmd = update(AddFavoriteEvent::Submit(Box::new(geo2.clone())), &mut model);
        let event = cmd.events().next().unwrap();
        if let Event::Favorites(event) = &event {
            assert!(matches!(**event, FavoritesEvent::Set));
        } else {
            panic!("Expected Favorites event")
        }
        assert_eq!(model.favorites.len(), 2);
        assert_eq!(model.favorites[1].geo, geo2);

        // Verify both favorites are in the list
        assert_eq!(model.favorites[0].geo, geo1);
        assert_eq!(model.favorites[1].geo, geo2);
        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::Idle)
        ));
    }

    #[test]
    fn test_search_triggers_api_call() {
        let app = App;
        let mut model = Model::default();

        let query = "Phoenix";
        let event = Event::AddFavorite(Box::new(AddFavoriteEvent::Search(query.to_string())));

        let mut cmd = app.update(event, &mut model, &());

        let mut request = cmd.effects().next().unwrap().expect_http();

        assert_eq!(
            &request.operation,
            &HttpRequest::get(GEOCODING_URL)
                .query(&GeocodingQueryString {
                    q: query.to_string(),
                    limit: "5",
                    appid: API_KEY.clone(),
                })
                .expect("could not serialize query string")
                .build()
        );

        // Test response handling
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(SAMPLE_GEOCODING_RESPONSE_JSON.as_bytes())
                    .build(),
            ))
            .unwrap();

        let actual = cmd.events().next().unwrap();
        if let Event::AddFavorite(event) = &actual {
            assert!(matches!(**event, AddFavoriteEvent::SearchResult(_)));
        } else {
            panic!("Expected AddFavorite event")
        }

        // Send the SearchResult event back to the app
        let mut cmd = app.update(actual, &mut model, &());
        assert_effect!(cmd, Effect::Render(_));
        assert_eq!(
            model.search_results,
            Some(SAMPLE_GEOCODING_RESPONSE.clone())
        );
        insta::assert_yaml_snapshot!(model.search_results);
    }

    #[test]
    fn test_submit_duplicate_favorite() {
        let mut model = crate::Model::default();
        let geo = test_geocoding();

        // First submit - should succeed
        let mut cmd = update(AddFavoriteEvent::Submit(Box::new(geo.clone())), &mut model);

        // Verify first submit worked
        let event = cmd.events().next().unwrap();
        if let Event::Favorites(event) = &event {
            assert!(matches!(**event, FavoritesEvent::Set));
        } else {
            panic!("Expected Favorites event")
        }
        assert_eq!(model.favorites.len(), 1);
        assert_eq!(model.favorites[0].geo, geo);

        // Try to submit the same favorite again
        let mut cmd = update(AddFavoriteEvent::Submit(Box::new(geo.clone())), &mut model);

        // Verify no Set event was generated (no storage update)
        assert!(cmd.events().next().is_none());

        // Verify favorites list is unchanged
        assert_eq!(model.favorites.len(), 1);
        assert_eq!(model.favorites[0].geo, geo);

        // Verify we still return to favorites view
        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::Idle)
        ));
    }
}
