use crux_core::{render::render, Command};
use crux_kv::{command::KeyValue, error::KeyValueError};
use serde::{Deserialize, Serialize};
use serde_json;

use crate::favorites::model::{Favorite, Favorites, FavoritesState, FAVORITES_KEY};
use crate::location::client::{LocationApi, LocationError};
use crate::location::model::geocoding_response::GeocodingResponse;

use crate::location::Location;
use crate::{Effect, Workflow};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum FavoritesEvent {
    // Workflow - Favorites view
    DeletePressed(Location),
    DeleteConfirmed,
    DeleteCancelled,

    // Workflow - Add Favorite view
    Search(String),
    Submit(Box<GeocodingResponse>),
    Cancel,
    #[serde(skip)]
    SearchResult(Box<Result<Vec<GeocodingResponse>, LocationError>>),
    // KV Related
    Restore,
    #[serde(skip)]
    Set,
    #[serde(skip)]
    Stored(Result<Option<Vec<u8>>, KeyValueError>),
    #[serde(skip)]
    Load(Result<Option<Vec<u8>>, KeyValueError>),
}

pub fn update(event: FavoritesEvent, model: &mut crate::Model) -> Command<Effect, FavoritesEvent> {
    match event {
        FavoritesEvent::DeletePressed(location) => {
            model.page = Workflow::Favorites(FavoritesState::ConfirmDelete(location));
            render()
        }

        FavoritesEvent::DeleteConfirmed => {
            if let Workflow::Favorites(FavoritesState::ConfirmDelete(location)) = model.page {
                model.page = Workflow::Favorites(FavoritesState::Idle);

                if model.favorites.remove(&location).is_some() {
                    return update(FavoritesEvent::Set, model).and(render());
                }
            }

            render()
        }

        FavoritesEvent::DeleteCancelled => {
            model.page = Workflow::Favorites(FavoritesState::Idle);
            render()
        }

        // ======================
        // Workflow - Add Favorite view
        // ======================
        // TODO: use a Time Capability and debounce the search
        // TODO: Search should be a part of events/geocoding.rs
        FavoritesEvent::Search(query) => LocationApi::fetch(&query)
            .then_send(|result| FavoritesEvent::SearchResult(Box::new(result))),
        FavoritesEvent::SearchResult(result) => {
            match *result {
                Ok(results) => {
                    model.search_results = Some(results);
                }
                Err(_) => {
                    model.search_results = Some(Vec::new());
                }
            }
            render()
        }
        FavoritesEvent::Submit(geo) => {
            model.page = Workflow::Favorites(FavoritesState::Idle);
            model.search_results = None;

            let favorite = Favorite::from(*geo);

            // Check if a favorite with the same coordinates already exists
            if model.favorites.exists(&favorite.location()) {
                // If it's a duplicate, just return to favorites view without adding
                // TODO: show a toast message
                render()
            } else {
                model.favorites.insert(favorite.clone());
                render().and(Command::event(FavoritesEvent::Set))
            }
        }
        FavoritesEvent::Cancel => {
            model.search_results = None;
            model.page = Workflow::Favorites(FavoritesState::Idle);
            render()
        }
        // ======================
        // KV Storage Operations
        // ======================
        FavoritesEvent::Restore => KeyValue::get(FAVORITES_KEY).then_send(FavoritesEvent::Load),

        FavoritesEvent::Set => KeyValue::set(
            FAVORITES_KEY,
            serde_json::to_vec(model.favorites.as_slice()).unwrap(),
        )
        .then_send(FavoritesEvent::Stored),

        FavoritesEvent::Stored(result) => {
            match result {
                Ok(_) => {
                    println!("Stored!");
                }
                Err(err) => {
                    println!("Storing KV error: {err}");
                }
            }
            Command::done()
        }

        FavoritesEvent::Load(result) => {
            let Ok(Some(favorites_bytes)) = result else {
                return Command::done();
            };
            let Ok(favorites) = serde_json::from_slice::<Vec<Favorite>>(&favorites_bytes) else {
                return Command::done();
            };

            println!("Favorites are: {favorites:#?}");
            model.favorites = Favorites::from_vec(favorites);

            Command::done()
        }
    }
}

#[cfg(test)]
mod tests {
    use crux_core::{assert_effect, App as _};
    use crux_http::protocol::{HttpResponse, HttpResult};

    use super::*;
    use crate::{
        location::client::LocationApi,
        weather::model::{
            current_response::{Main, Sys},
            response_elements::{Clouds, Coord, WeatherData, Wind},
            CurrentResponse,
        },
        App, Effect, Event, GeocodingResponse, Model,
    };

    // Helper to create a test favorite
    fn test_favorite() -> Favorite {
        Favorite {
            geo: GeocodingResponse {
                name: "Phoenix".to_string(),
                local_names: None,
                lat: 33.456_789,
                lon: -112.037_222,
                country: "US".to_string(),
                state: None,
            },
            current: None,
        }
    }

    fn sample_geocoding_response() -> Vec<GeocodingResponse> {
        vec![GeocodingResponse {
            name: "Phoenix".to_string(),
            local_names: None,
            lat: 33.456_789,
            lon: -112.037_222,
            country: "US".to_string(),
            state: Some("Arizona".to_string()),
        }]
    }

    fn sample_geocoding_response_json() -> String {
        serde_json::to_string(&sample_geocoding_response()).unwrap()
    }

    #[test]
    fn test_kv_set_and_load() {
        // Model will have no favorites set
        let mut model = crate::Model::default();

        let favorites = Favorites::from_vec(vec![test_favorite()]);

        let mut cmd = update(
            FavoritesEvent::Load(Ok(Some(serde_json::to_vec(favorites.as_slice()).unwrap()))),
            &mut model,
        );
        assert!(cmd.effects().next().is_none());
        assert_eq!(model.favorites, favorites);
    }

    #[test]
    fn test_kv_load_empty() {
        let mut model = crate::Model::default();
        let mut cmd = update(FavoritesEvent::Load(Ok(None)), &mut model);
        assert!(cmd.effects().next().is_none());
        assert!(model.favorites.is_empty());
    }

    #[test]
    fn test_kv_load_error() {
        let mut model = crate::Model::default();
        let mut cmd = update(
            FavoritesEvent::Load(Err(KeyValueError::CursorNotFound)),
            &mut model,
        );
        assert!(cmd.effects().next().is_none());
        assert!(model.favorites.is_empty());
    }

    #[test]
    fn test_delete_with_persistence() {
        let mut model = crate::Model::default();
        let favorite = test_favorite();
        let favorite = favorite.clone(); // Clone once at the start
        model.favorites.insert(favorite.clone());

        // Set the state to ConfirmDelete with the favorite's coordinates
        model.page = Workflow::Favorites(FavoritesState::ConfirmDelete(Location {
            lat: favorite.geo.lat,
            lon: favorite.geo.lon,
        }));

        // Delete and verify KV is updated
        let mut cmd = update(FavoritesEvent::DeleteConfirmed, &mut model);
        let effect = cmd.effects().next().unwrap();
        assert!(matches!(effect, Effect::KeyValue(_)));

        // Verify the empty state persists
        let mut cmd = update(
            FavoritesEvent::Load(Ok(Some(
                serde_json::to_vec(model.favorites.as_slice()).unwrap(),
            ))),
            &mut model,
        );

        assert!(cmd.effects().next().is_none());
        assert!(model.favorites.is_empty());
    }

    #[test]
    fn test_delete_pressed() {
        let mut model = crate::Model::default();
        let favorite = Favorite {
            geo: GeocodingResponse {
                name: "Phoenix".to_string(),
                local_names: None,
                lat: 33.456_789,
                lon: -112.037_222,
                country: "US".to_string(),
                state: None,
            },
            current: None,
        };

        let _ = update(
            FavoritesEvent::DeletePressed(Location {
                lat: favorite.geo.lat,
                lon: favorite.geo.lon,
            }),
            &mut model,
        );

        // Verify the state was updated correctly
        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::ConfirmDelete(Location {
                lat: 33.456_789,
                lon: -112.037_222,
            }))
        ));
    }

    #[test]
    fn test_delete_confirmed() {
        let app = crate::App;
        let mut model = crate::Model::default();
        let favorite = Favorite {
            geo: GeocodingResponse {
                name: "Phoenix".to_string(),
                local_names: None,
                lat: 33.456_789,
                lon: -112.037_222,
                country: "US".to_string(),
                state: None,
            },
            current: Some(CurrentResponse {
                coord: Coord {
                    lat: 33.456_789,
                    lon: -112.037_222,
                },
                weather: vec![WeatherData {
                    id: 800,
                    main: "Clear".to_string(),
                    description: "clear sky".to_string(),
                    icon: "01d".to_string(),
                }],
                base: "stations".to_string(),
                main: Main {
                    temp: 20.0,
                    feels_like: 18.0,
                    temp_min: 18.0,
                    temp_max: 22.0,
                    pressure: 1013,
                    humidity: 50,
                },
                visibility: 10000,
                wind: Wind {
                    speed: 4.1,
                    deg: 280,
                    gust: Some(5.2),
                },
                clouds: Clouds { all: 0 },
                dt: 1_716_216_000,
                sys: Sys {
                    id: 1,
                    country: "US".to_string(),
                    sys_type: 1,
                    sunrise: 1_716_216_000,
                    sunset: 1_716_216_000,
                },
                timezone: 1,
                id: 1,
                name: "Phoenix".to_string(),
                cod: 200,
            }),
        };

        let latlon = Location {
            lat: favorite.geo.lat,
            lon: favorite.geo.lon,
        };

        model.favorites.insert(favorite.clone());
        model.page = Workflow::Favorites(FavoritesState::ConfirmDelete(latlon));

        // First command from DeleteConfirmed
        let mut cmd = app.update(
            Event::Favorites(Box::new(FavoritesEvent::DeleteConfirmed)),
            &mut model,
        );

        let effect = cmd.effects().next().unwrap();
        assert!(matches!(effect, Effect::KeyValue(_)));

        // Verify the favorite was removed and state was reset
        assert!(model.favorites.is_empty());
        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::Idle)
        ));
    }

    #[test]
    fn test_delete_cancelled() {
        let mut model = Model {
            page: Workflow::Favorites(FavoritesState::ConfirmDelete(Location {
                lat: 33.456_789,
                lon: 112.037_222,
            })),
            ..Default::default()
        };

        let _ = update(FavoritesEvent::DeleteCancelled, &mut model);

        // Verify the state was reset
        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::Idle)
        ));
    }

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
        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo.clone())), &mut model);

        // Verify we get the Set event
        let event = cmd.events().next().unwrap();
        assert!(matches!(event, FavoritesEvent::Set));

        // Verify the favorite was added and state was updated
        assert_eq!(model.favorites.len(), 1);
        assert_eq!(model.favorites.get(&geo.location()).unwrap().geo, geo);
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

        let _ = update(FavoritesEvent::Cancel, &mut model);

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
        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo1.clone())), &mut model);
        let event = cmd.events().next().unwrap();
        assert!(matches!(event, FavoritesEvent::Set));
        assert_eq!(model.favorites.len(), 1);
        assert_eq!(model.favorites.get(&geo1.location()).unwrap().geo, geo1);

        // Submit second favorite (different location)
        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo2.clone())), &mut model);
        let event = cmd.events().next().unwrap();
        assert!(matches!(event, FavoritesEvent::Set));

        // Verify both favorites are in the list
        assert_eq!(model.favorites.len(), 2);
        assert_eq!(model.favorites.get(&geo1.location()).unwrap().geo, geo1);
        assert_eq!(model.favorites.get(&geo2.location()).unwrap().geo, geo2);

        // Verify we can't add the same favorite again
        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo1.clone())), &mut model);
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
        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo1.clone())), &mut model);
        let event = cmd.events().next().unwrap();
        assert!(matches!(event, FavoritesEvent::Set));
        assert_eq!(model.favorites.len(), 1);
        assert_eq!(model.favorites.get(&geo1.location()).unwrap().geo, geo1);

        // Add second favorite
        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo2.clone())), &mut model);
        let event = cmd.events().next().unwrap();
        assert!(matches!(event, FavoritesEvent::Set));
        assert_eq!(model.favorites.len(), 2);
        assert_eq!(model.favorites.get(&geo2.location()).unwrap().geo, geo2);

        // Verify both favorites are in the list
        assert_eq!(model.favorites.get(&geo1.location()).unwrap().geo, geo1);
        assert_eq!(model.favorites.get(&geo2.location()).unwrap().geo, geo2);
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
        let event = Event::Favorites(Box::new(FavoritesEvent::Search(query.to_string())));

        let mut cmd = app.update(event, &mut model);

        let mut request = cmd.effects().next().unwrap().expect_http();

        assert_eq!(&request.operation, &LocationApi::build(query));

        // Test response handling
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(sample_geocoding_response_json().as_bytes())
                    .build(),
            ))
            .unwrap();

        let actual = cmd.events().next().unwrap();
        if let Event::Favorites(event) = &actual {
            assert!(matches!(**event, FavoritesEvent::SearchResult(_)));
        } else {
            panic!("Expected Favorites event")
        }

        // Send the SearchResult event back to the app
        let mut cmd = app.update(actual, &mut model);
        assert_effect!(cmd, Effect::Render(_));
        assert_eq!(
            model.search_results,
            Some(sample_geocoding_response().clone())
        );
        insta::assert_yaml_snapshot!(model.search_results);
    }

    #[test]
    fn test_submit_duplicate_favorite() {
        let mut model = crate::Model::default();
        let geo = test_geocoding();

        // First submit - should succeed
        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo.clone())), &mut model);

        // Verify first submit worked
        let event = cmd.events().next().unwrap();
        assert!(matches!(event, FavoritesEvent::Set));
        assert_eq!(model.favorites.len(), 1);
        assert_eq!(model.favorites.get(&geo.location()).unwrap().geo, geo);

        // Try to submit the same favorite again
        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo.clone())), &mut model);

        // Verify no Set event was generated (no storage update)
        assert!(cmd.events().next().is_none());

        // Verify favorites list is unchanged
        assert_eq!(model.favorites.len(), 1);
        assert_eq!(model.favorites.get(&geo.location()).unwrap().geo, geo);

        // Verify we still return to favorites view
        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::Idle)
        ));
    }
}
