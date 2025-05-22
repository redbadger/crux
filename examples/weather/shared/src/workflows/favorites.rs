use crux_core::{render::render, Command};
use serde::{Deserialize, Serialize};

use crate::{CurrentResponse, Effect, Event, GeocodingResponse, Workflow};

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
    DeletePressed(Favorite),
    DeleteConfirmed,
    DeleteCancelled,
}

pub fn update(event: FavoritesEvent, model: &mut crate::Model) -> Command<Effect, Event> {
    match event {
        FavoritesEvent::AddPressed => {
            model.page = Workflow::AddFavorite;
            Command::done()
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
                }
            }
            model.page = Workflow::Favorites(FavoritesState::Idle);
            render()
        }

        FavoritesEvent::DeleteCancelled => {
            model.page = Workflow::Favorites(FavoritesState::Idle);
            render()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Clouds, Coord, CurrentResponse, Effect, GeocodingResponse, Main, Sys, Weather, Wind,
    };

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

        let mut cmd = update(FavoritesEvent::DeletePressed(favorite.clone()), &mut model);
        assert!(matches!(cmd.effects().next(), Some(Effect::Render(_)))); // Should have a render effect

        // Verify the state was updated correctly
        assert!(matches!(
            model.page,
            Workflow::Favorites(FavoritesState::ConfirmDelete(33.456789, -112.037222))
        ));
    }

    #[test]
    fn test_delete_confirmed() {
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
                weather: vec![Weather {
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

        model.favorites.push(favorite);
        model.page = Workflow::Favorites(FavoritesState::ConfirmDelete(33.456789, -112.037222));

        let mut cmd = update(FavoritesEvent::DeleteConfirmed, &mut model);
        assert!(matches!(cmd.effects().next(), Some(Effect::Render(_))));

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
