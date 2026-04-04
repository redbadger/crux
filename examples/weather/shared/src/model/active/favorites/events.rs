use crux_core::{Command, render::render};
use crux_kv::{command::KeyValue, error::KeyValueError};
use facet::Facet;
use serde::{Deserialize, Serialize};
use serde_json;

use crate::effects::{
    Effect,
    location::Location,
};
use crate::model::ApiKey;
use super::client::{LocationApi, LocationError};
use super::super::location::GeocodingResponse;
use super::{FavoritesScreen, FavoritesWorkflow};
use super::model::{FAVORITES_KEY, Favorite, Favorites};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum FavoritesEvent {
    // Navigation
    GoToHome,
    GoToAddFavorite,

    // Favorites view
    DeletePressed(Location),
    DeleteConfirmed,
    DeleteCancelled,

    // Add Favorite view
    Search(String),
    Submit(Box<GeocodingResponse>),
    Cancel,

    #[serde(skip)]
    #[facet(skip)]
    SearchResult(#[facet(opaque)] Box<Result<Vec<GeocodingResponse>, LocationError>>),

    // KV Related
    Restore,

    #[serde(skip)]
    #[facet(skip)]
    Set,

    #[serde(skip)]
    #[facet(skip)]
    Stored(#[facet(opaque)] Result<Option<Vec<u8>>, KeyValueError>),

    #[serde(skip)]
    #[facet(skip)]
    Load(#[facet(opaque)] Result<Option<Vec<u8>>, KeyValueError>),
}

pub fn update(event: FavoritesEvent, screen: &mut FavoritesScreen, api_key: &ApiKey) -> Command<Effect, FavoritesEvent> {
    match event {
        // GoToHome and GoToAddFavorite are handled by FavoritesScreen::update
        FavoritesEvent::GoToHome | FavoritesEvent::GoToAddFavorite => {
            unreachable!("handled by FavoritesScreen::update")
        }

        FavoritesEvent::DeletePressed(location) => {
            screen.workflow = Some(FavoritesWorkflow::ConfirmDelete(location));
            render()
        }

        FavoritesEvent::DeleteConfirmed => {
            if let Some(FavoritesWorkflow::ConfirmDelete(location)) = screen.workflow {
                screen.workflow = None;

                if screen.favorites.remove(&location).is_some() {
                    return update(FavoritesEvent::Set, screen, api_key).and(render());
                }
            }

            render()
        }

        FavoritesEvent::DeleteCancelled => {
            screen.workflow = None;
            render()
        }

        FavoritesEvent::Search(query) => LocationApi::fetch(&query, api_key.clone())
            .then_send(|result| FavoritesEvent::SearchResult(Box::new(result))),

        FavoritesEvent::SearchResult(result) => {
            if let Some(FavoritesWorkflow::AddFavorite { ref mut search_results }) = screen.workflow {
                *search_results = Some(match *result {
                    Ok(results) => results,
                    Err(_) => Vec::new(),
                });
            }
            render()
        }

        FavoritesEvent::Submit(geo) => {
            screen.workflow = None;

            let favorite = Favorite::from(*geo);

            if screen.favorites.exists(&favorite.location()) {
                render()
            } else {
                screen.favorites.insert(favorite.clone());
                render().and(Command::event(FavoritesEvent::Set))
            }
        }

        FavoritesEvent::Cancel => {
            screen.workflow = None;
            render()
        }

        FavoritesEvent::Restore => {
            KeyValue::get(FAVORITES_KEY).then_send(FavoritesEvent::Load)
        }

        FavoritesEvent::Set => KeyValue::set(
            FAVORITES_KEY,
            serde_json::to_vec(screen.favorites.as_slice()).unwrap(),
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
            screen.favorites = Favorites::from_vec(favorites);

            Command::done()
        }
    }
}

#[cfg(test)]
mod tests {
    use crux_core::{App as _, assert_effect};
    use crux_http::protocol::{HttpResponse, HttpResult};
    use crux_kv::KeyValueOperation;

    use super::*;
    use crate::{
        app::Weather,
        effects::Effect,
        model::{ActiveEvent, ActiveModel, Event, Model},
    };
    use crate::model::active::Screen;
    use crate::model::active::favorites::client::LocationApi;

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

    // Helper to create a test favorite
    fn test_favorite() -> Favorite {
        Favorite(GeocodingResponse {
            name: "Phoenix".to_string(),
            local_names: None,
            lat: 33.456_789,
            lon: -112.037_222,
            country: "US".to_string(),
            state: None,
        })
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
    fn kv_set_and_load() {
        let mut screen = test_screen();
        let api_key = test_api_key();

        let favorites = Favorites::from_vec(vec![test_favorite()]);

        let mut cmd = update(
            FavoritesEvent::Load(Ok(Some(serde_json::to_vec(favorites.as_slice()).unwrap()))),
            &mut screen,
            &api_key,
        );
        assert!(cmd.effects().next().is_none());
        assert_eq!(screen.favorites, favorites);
    }

    #[test]
    fn kv_load_empty() {
        let mut screen = test_screen();
        let api_key = test_api_key();
        let mut cmd = update(FavoritesEvent::Load(Ok(None)), &mut screen, &api_key);
        assert!(cmd.effects().next().is_none());
        assert!(screen.favorites.is_empty());
    }

    #[test]
    fn kv_load_error() {
        let mut screen = test_screen();
        let api_key = test_api_key();
        let mut cmd = update(
            FavoritesEvent::Load(Err(KeyValueError::CursorNotFound)),
            &mut screen,
            &api_key,
        );
        assert!(cmd.effects().next().is_none());
        assert!(screen.favorites.is_empty());
    }

    #[test]
    fn delete_with_persistence() {
        let mut screen = test_screen();
        let api_key = test_api_key();
        let favorite = test_favorite();
        screen.favorites.insert(favorite.clone());

        screen.workflow = Some(FavoritesWorkflow::ConfirmDelete(favorite.location()));

        let mut cmd = update(FavoritesEvent::DeleteConfirmed, &mut screen, &api_key);
        let kv_request = cmd.expect_effect().expect_key_value();
        cmd.expect_one_effect().expect_render();

        assert!(matches!(
            kv_request.operation,
            KeyValueOperation::Set { .. }
        ));

        assert!(screen.favorites.is_empty());

        cmd.expect_no_effects();
        cmd.expect_no_events();
    }

    #[test]
    fn delete_pressed() {
        let mut screen = test_screen();
        let api_key = test_api_key();

        let _ = update(
            FavoritesEvent::DeletePressed(Location {
                lat: 33.456_789,
                lon: -112.037_222,
            }),
            &mut screen,
            &api_key,
        );

        assert!(matches!(
            screen.workflow,
            Some(FavoritesWorkflow::ConfirmDelete(Location {
                lat: 33.456_789,
                lon: -112.037_222,
            }))
        ));
    }

    #[test]
    fn delete_confirmed_integration() {
        let app = Weather;
        let favorite = Favorite(GeocodingResponse {
            name: "Phoenix".to_string(),
            local_names: None,
            lat: 33.456_789,
            lon: -112.037_222,
            country: "US".to_string(),
            state: None,
        });

        let latlon = favorite.location();

        let mut favorites = Favorites::default();
        favorites.insert(favorite.clone());
        let mut model = Model::Active(ActiveModel {
            api_key: test_api_key(),
            screen: Screen::Favorites(FavoritesScreen {
                favorites,
                workflow: Some(FavoritesWorkflow::ConfirmDelete(latlon)),
            }),
        });

        let mut cmd = app.update(
            Event::Active(ActiveEvent::Favorites(Box::new(FavoritesEvent::DeleteConfirmed))),
            &mut model,
        );

        let effect = cmd.effects().next().unwrap();
        assert!(matches!(effect, Effect::KeyValue(_)));

        let Model::Active(active) = &model else {
            panic!("Expected Active");
        };
        let Screen::Favorites(fav) = &active.screen else {
            panic!("Expected Favorites screen");
        };
        assert!(fav.favorites.is_empty());
        assert!(fav.workflow.is_none());
    }

    #[test]
    fn delete_cancelled() {
        let mut screen = FavoritesScreen {
            favorites: Favorites::default(),
            workflow: Some(FavoritesWorkflow::ConfirmDelete(Location {
                lat: 33.456_789,
                lon: 112.037_222,
            })),
        };
        let api_key = test_api_key();

        let _ = update(FavoritesEvent::DeleteCancelled, &mut screen, &api_key);

        assert!(screen.workflow.is_none());
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
    fn submit_adds_favorite() {
        let mut screen = test_screen();
        let api_key = test_api_key();
        let geo = test_geocoding();

        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo.clone())), &mut screen, &api_key);

        let event = cmd.events().next().unwrap();
        assert!(matches!(event, FavoritesEvent::Set));

        assert_eq!(screen.favorites.len(), 1);
        assert_eq!(screen.favorites.get(&geo.location()).unwrap().0, geo);
        assert!(screen.workflow.is_none());
    }

    #[test]
    fn cancel_returns_to_favorites() {
        let mut screen = FavoritesScreen {
            favorites: Favorites::default(),
            workflow: Some(FavoritesWorkflow::AddFavorite { search_results: None }),
        };
        let api_key = test_api_key();

        let _ = update(FavoritesEvent::Cancel, &mut screen, &api_key);

        assert!(screen.workflow.is_none());
        assert!(screen.favorites.is_empty());
    }

    #[test]
    fn submit_persists_favorite() {
        let mut screen = test_screen();
        let api_key = test_api_key();
        let geo1 = test_geocoding();
        let geo2 = GeocodingResponse {
            name: "New York".to_string(),
            local_names: None,
            lat: 40.7128,
            lon: -74.0060,
            country: "US".to_string(),
            state: None,
        };

        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo1.clone())), &mut screen, &api_key);
        let event = cmd.events().next().unwrap();
        assert!(matches!(event, FavoritesEvent::Set));
        assert_eq!(screen.favorites.len(), 1);
        assert_eq!(screen.favorites.get(&geo1.location()).unwrap().0, geo1);

        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo2.clone())), &mut screen, &api_key);
        let event = cmd.events().next().unwrap();
        assert!(matches!(event, FavoritesEvent::Set));

        assert_eq!(screen.favorites.len(), 2);
        assert_eq!(screen.favorites.get(&geo1.location()).unwrap().0, geo1);
        assert_eq!(screen.favorites.get(&geo2.location()).unwrap().0, geo2);

        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo1.clone())), &mut screen, &api_key);
        assert!(cmd.events().next().is_none());
        assert_eq!(screen.favorites.len(), 2);
    }

    #[test]
    fn add_multiple_favorites() {
        let mut screen = test_screen();
        let api_key = test_api_key();
        let geo1 = test_geocoding();
        let geo2 = GeocodingResponse {
            name: "New York".to_string(),
            local_names: None,
            lat: 40.7128,
            lon: -74.0060,
            country: "US".to_string(),
            state: None,
        };

        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo1.clone())), &mut screen, &api_key);
        let event = cmd.events().next().unwrap();
        assert!(matches!(event, FavoritesEvent::Set));
        assert_eq!(screen.favorites.len(), 1);
        assert_eq!(screen.favorites.get(&geo1.location()).unwrap().0, geo1);

        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo2.clone())), &mut screen, &api_key);
        let event = cmd.events().next().unwrap();
        assert!(matches!(event, FavoritesEvent::Set));
        assert_eq!(screen.favorites.len(), 2);
        assert_eq!(screen.favorites.get(&geo2.location()).unwrap().0, geo2);

        assert_eq!(screen.favorites.get(&geo1.location()).unwrap().0, geo1);
        assert_eq!(screen.favorites.get(&geo2.location()).unwrap().0, geo2);
        assert!(screen.workflow.is_none());
    }

    #[test]
    fn search_triggers_api_call() {
        let app = Weather;
        let mut model = Model::Active(ActiveModel {
            api_key: test_api_key(),
            screen: Screen::Favorites(FavoritesScreen {
                favorites: Favorites::default(),
                workflow: Some(FavoritesWorkflow::AddFavorite { search_results: None }),
            }),
        });

        let query = "Phoenix";
        let event = Event::Active(ActiveEvent::Favorites(Box::new(
            FavoritesEvent::Search(query.to_string()),
        )));

        let mut cmd = app.update(event, &mut model);

        let mut request = cmd.effects().next().unwrap().expect_http();

        assert_eq!(
            &request.operation,
            &LocationApi::build(query, &test_api_key())
        );

        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(sample_geocoding_response_json().as_bytes())
                    .build(),
            ))
            .unwrap();

        let actual = cmd.events().next().unwrap();
        if let Event::Active(ActiveEvent::Favorites(event)) = &actual {
            assert!(matches!(**event, FavoritesEvent::SearchResult(_)));
        } else {
            panic!("Expected Active(Favorites) event")
        }

        let mut cmd = app.update(actual, &mut model);
        assert_effect!(cmd, Effect::Render(_));
        let Model::Active(active) = &model else {
            panic!("Expected Active");
        };
        let Screen::Favorites(fav) = &active.screen else {
            panic!("Expected Favorites screen");
        };
        let Some(FavoritesWorkflow::AddFavorite { ref search_results }) = fav.workflow else {
            panic!("Expected AddFavorite workflow");
        };
        assert_eq!(
            *search_results,
            Some(sample_geocoding_response().clone())
        );
        insta::assert_yaml_snapshot!(search_results);
    }

    #[test]
    fn submit_duplicate_favorite() {
        let mut screen = test_screen();
        let api_key = test_api_key();
        let geo = test_geocoding();

        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo.clone())), &mut screen, &api_key);

        let event = cmd.events().next().unwrap();
        assert!(matches!(event, FavoritesEvent::Set));
        assert_eq!(screen.favorites.len(), 1);
        assert_eq!(screen.favorites.get(&geo.location()).unwrap().0, geo);

        let mut cmd = update(FavoritesEvent::Submit(Box::new(geo.clone())), &mut screen, &api_key);

        assert!(cmd.events().next().is_none());

        assert_eq!(screen.favorites.len(), 1);
        assert_eq!(screen.favorites.get(&geo.location()).unwrap().0, geo);

        assert!(screen.workflow.is_none());
    }
}
