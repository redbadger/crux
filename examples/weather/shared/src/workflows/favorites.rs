use crux_core::{render::render, Command};
use crux_kv::{command::KeyValue, error::KeyValueError};
use serde::{Deserialize, Serialize};
use serde_json;

use crate::{CurrentResponse, Effect, Event, GeocodingResponse, Workflow};

const FAVORITES_KEY: &str = "favorites";

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct Favorite {
    pub geo: GeocodingResponse,
    pub current: Option<CurrentResponse>,
}

impl From<GeocodingResponse> for Favorite {
    fn from(geo: GeocodingResponse) -> Self {
        Favorite { geo, current: None }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum FavoritesState {
    Idle,
    ConfirmDelete(f64, f64),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum FavoritesEvent {
    AddPressed,
    DeletePressed(Box<Favorite>),
    DeleteConfirmed,
    DeleteCancelled,
    // KV Related
    Restore,
    #[serde(skip)]
    Set,
    #[serde(skip)]
    Load(Result<Option<Vec<u8>>, KeyValueError>),
}

pub fn update(event: FavoritesEvent, model: &mut crate::Model) -> Command<Effect, Event> {
    match event {
        FavoritesEvent::AddPressed => {
            model.page = Workflow::AddFavorite;
            render()
        }

        FavoritesEvent::DeletePressed(favorite) => {
            model.page = Workflow::Favorites(FavoritesState::ConfirmDelete(
                favorite.geo.lat,
                favorite.geo.lon,
            ));
            render()
        }

        FavoritesEvent::DeleteConfirmed => {
            if let Workflow::Favorites(FavoritesState::ConfirmDelete(lat, lng)) = model.page {
                if let Some(index) = model
                    .favorites
                    .iter()
                    .position(|f| f.geo.lat == lat && f.geo.lon == lng)
                {
                    model.favorites.remove(index);
                    model.page = Workflow::Favorites(FavoritesState::Idle);
                    Command::event(Event::Favorites(Box::new(FavoritesEvent::Set)))
                } else {
                    model.page = Workflow::Favorites(FavoritesState::Idle);
                    render()
                }
            } else {
                render()
            }
        }

        FavoritesEvent::DeleteCancelled => {
            model.page = Workflow::Favorites(FavoritesState::Idle);
            render()
        }

        // ======================
        // KV Storage Operations
        // ======================
        FavoritesEvent::Restore => KeyValue::get(FAVORITES_KEY)
            .then_send(|r| Event::Favorites(Box::new(FavoritesEvent::Load(r)))),

        FavoritesEvent::Set => {
            KeyValue::set(FAVORITES_KEY, serde_json::to_vec(&model.favorites).unwrap())
                .then_send(|_| Event::Render)
        }

        FavoritesEvent::Load(result) => match result {
            Ok(Some(favorites_bytes)) => {
                match serde_json::from_slice::<Vec<Favorite>>(&favorites_bytes) {
                    Ok(favorites) => {
                        println!("Favorites are: {:#?}", favorites);
                        model.favorites = favorites;
                        Command::done()
                    }
                    Err(_) => Command::done(),
                }
            }
            _ => Command::done(),
        },
    }
}

#[cfg(test)]
mod tests {
    use crux_core::App as _;

    use super::*;
    use crate::{
        Clouds, Coord, CurrentResponse, Effect, GeocodingResponse, Main, Sys, WeatherData, Wind,
    };

    // Helper to create a test favorite
    fn test_favorite() -> Favorite {
        Favorite {
            geo: GeocodingResponse {
                name: "Phoenix".to_string(),
                local_names: None,
                lat: 33.456789,
                lon: -112.037222,
                country: "US".to_string(),
                state: None,
            },
            current: None,
        }
    }

    #[test]
    fn test_kv_set_and_load() {
        // Model will have no favorites set
        let mut model = crate::Model::default();

        let favorites = vec![test_favorite()];

        let mut cmd = update(
            FavoritesEvent::Load(Ok(Some(serde_json::to_vec(&favorites).unwrap()))),
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
        model.favorites.push(favorite.clone());

        // Set the state to ConfirmDelete with the favorite's coordinates
        model.page = Workflow::Favorites(FavoritesState::ConfirmDelete(
            favorite.geo.lat,
            favorite.geo.lon,
        ));

        // Delete and verify KV is updated
        let mut cmd = update(FavoritesEvent::DeleteConfirmed, &mut model);

        // Verify we get the Set event first
        let event = cmd.events().next().unwrap();
        if let Event::Favorites(event) = &event {
            assert!(matches!(**event, FavoritesEvent::Set))
        } else {
            panic!("Expected Favorites event")
        }
        assert!(model.favorites.is_empty());

        // Process the Set event to get the KeyValue effect
        let mut cmd = update(FavoritesEvent::Set, &mut model);
        let effect = cmd.effects().next().unwrap();
        assert!(matches!(effect, Effect::KeyValue(_)));

        // Verify the empty state persists
        let mut cmd = update(
            FavoritesEvent::Load(Ok(Some(serde_json::to_vec(&model.favorites).unwrap()))),
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
                lat: 33.456789,
                lon: -112.037222,
                country: "US".to_string(),
                state: None,
            },
            current: None,
        };

        let mut cmd = update(
            FavoritesEvent::DeletePressed(Box::new(favorite.clone())),
            &mut model,
        );
        assert!(matches!(cmd.effects().next(), Some(Effect::Render(_)))); // Should have a render effect

        // Verify the state was updated correctly
        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::ConfirmDelete(33.456789, -112.037222))
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
                lat: 33.456789,
                lon: -112.037222,
                country: "US".to_string(),
                state: None,
            },
            current: Some(CurrentResponse {
                coord: Coord {
                    lat: 33.456789,
                    lon: -112.037222,
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
                dt: 1716216000,
                sys: Sys {
                    id: 1,
                    country: "US".to_string(),
                    sys_type: 1,
                    sunrise: 1716216000,
                    sunset: 1716216000,
                },
                timezone: 1,
                id: 1,
                name: "Phoenix".to_string(),
                cod: 200,
            }),
        };

        let latlng = (favorite.geo.lat, favorite.geo.lon);

        model.favorites.push(favorite.clone());
        model.page = Workflow::Favorites(FavoritesState::ConfirmDelete(latlng.0, latlng.1));

        // First command from DeleteConfirmed
        let mut cmd = app.update(
            Event::Favorites(Box::new(FavoritesEvent::DeleteConfirmed)),
            &mut model,
            &(),
        );

        // Verify we get the Set event first
        let event = cmd.events().next().unwrap();
        if let Event::Favorites(event) = &event {
            assert!(matches!(**event, FavoritesEvent::Set))
        } else {
            panic!("Expected Favorites event")
        }
        assert!(model.favorites.is_empty());

        let mut cmd = update(FavoritesEvent::Set, &mut model);
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
        let mut model = crate::Model::default();
        model.page = Workflow::Favorites(FavoritesState::ConfirmDelete(33.456789, -112.037222));

        let mut cmd = update(FavoritesEvent::DeleteCancelled, &mut model);
        assert!(matches!(cmd.effects().next(), Some(Effect::Render(_))));

        // Verify the state was reset
        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::Idle)
        ));
    }

    #[test]
    fn test_add_pressed() {
        let mut model = crate::Model::default();
        let mut cmd = update(FavoritesEvent::AddPressed, &mut model);
        assert!(cmd.effects().next().is_none());

        assert!(matches!(model.page, Workflow::AddFavorite));
    }
}
