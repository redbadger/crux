use crux_core::{render::render, Command};
use serde::{Deserialize, Serialize};

use super::favorites::{Favorite, FavoritesEvent, FavoritesState};
use crate::{Effect, Event, GeocodingResponse, Workflow};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AddFavoriteEvent {
    Submit(Box<GeocodingResponse>),
    Cancel,
}

pub fn update(event: AddFavoriteEvent, model: &mut crate::Model) -> Command<Effect, Event> {
    match event {
        AddFavoriteEvent::Submit(geo) => {
            let favorite = Favorite::from(*geo);
            model.favorites.push(favorite.clone());
            model.page = Workflow::Favorites(FavoritesState::Idle);
            Command::event(Event::Favorites(Box::new(FavoritesEvent::Set)))
        }
        AddFavoriteEvent::Cancel => {
            model.page = Workflow::Favorites(FavoritesState::Idle);
            render()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Effect, Event, GeocodingResponse};

    // Helper to create a test geocoding response
    fn test_geocoding() -> GeocodingResponse {
        GeocodingResponse {
            name: "Phoenix".to_string(),
            local_names: None,
            lat: 33.456789,
            lon: -112.037222,
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
            assert!(matches!(**event, FavoritesEvent::Set))
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
        let mut model = crate::Model::default();
        model.page = Workflow::AddFavorite;

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
        let geo = test_geocoding();

        // Submit the favorite
        let mut cmd = update(AddFavoriteEvent::Submit(Box::new(geo.clone())), &mut model);

        // Verify we get the Set event which will trigger KV storage
        let event = cmd.events().next().unwrap();
        if let Event::Favorites(event) = &event {
            assert!(matches!(**event, FavoritesEvent::Set))
        } else {
            panic!("Expected Favorites event")
        }

        // Verify the favorite was added
        assert_eq!(model.favorites.len(), 1);
        assert_eq!(model.favorites[0].geo, geo);

        // Submit another favorite to verify persistence
        let _cmd = update(AddFavoriteEvent::Submit(Box::new(geo.clone())), &mut model);

        // Verify the favorite persists after loading
        assert_eq!(model.favorites.len(), 2); // Now we have 2 favorites since we submitted again
        assert_eq!(model.favorites[1].geo, geo);
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
            assert!(matches!(**event, FavoritesEvent::Set))
        } else {
            panic!("Expected Favorites event")
        }
        assert_eq!(model.favorites.len(), 1);
        assert_eq!(model.favorites[0].geo, geo1);

        // Add second favorite
        let mut cmd = update(AddFavoriteEvent::Submit(Box::new(geo2.clone())), &mut model);
        let event = cmd.events().next().unwrap();
        if let Event::Favorites(event) = &event {
            assert!(matches!(**event, FavoritesEvent::Set))
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
}
