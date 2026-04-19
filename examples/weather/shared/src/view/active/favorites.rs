use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    effects::{http::location::GeocodingResponse, location::Location},
    model::active::favorites::{
        FavoritesScreen, FavoritesWorkflow, add::AddFavoriteWorkflow, model::Favorite,
    },
};

/// View model for the favourites screen.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct FavoritesViewModel {
    /// The saved favourites, in insertion order.
    pub favorites: Vec<FavoriteViewModel>,
    /// The active workflow, if any — add or confirm-delete.
    pub workflow: Option<FavoritesWorkflowViewModel>,
}

/// One row in the favourites list on the favourites screen.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FavoriteViewModel {
    pub name: String,
    pub location: Location,
}

/// The workflow currently layered on top of the favourites list.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum FavoritesWorkflowViewModel {
    /// Delete-confirmation dialog for the favourite at `location`.
    ConfirmDelete { location: Location },
    /// Add-favourite search screen.
    Add(AddFavoriteViewModel),
}

/// View model for the add-favourite search screen.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct AddFavoriteViewModel {
    /// The current contents of the search field.
    pub search_input: String,
    /// The latest search results, or `None` before any search has resolved.
    pub search_results: Option<Vec<GeocodingResponse>>,
    /// Whether a search is currently in flight (used to show a spinner).
    pub searching: bool,
}

impl From<&FavoritesScreen> for FavoritesViewModel {
    fn from(fav: &FavoritesScreen) -> Self {
        FavoritesViewModel {
            favorites: fav.favorites.iter().map(From::from).collect(),
            workflow: fav.workflow.as_ref().map(From::from),
        }
    }
}

impl From<&Favorite> for FavoriteViewModel {
    fn from(fav: &Favorite) -> Self {
        FavoriteViewModel {
            name: fav.name().to_string(),
            location: fav.location(),
        }
    }
}

impl From<&FavoritesWorkflow> for FavoritesWorkflowViewModel {
    fn from(wf: &FavoritesWorkflow) -> Self {
        match wf {
            FavoritesWorkflow::ConfirmDelete(cd) => FavoritesWorkflowViewModel::ConfirmDelete {
                location: cd.location,
            },
            FavoritesWorkflow::Add(add) => FavoritesWorkflowViewModel::Add(add.into()),
        }
    }
}

impl From<&AddFavoriteWorkflow> for AddFavoriteViewModel {
    fn from(add: &AddFavoriteWorkflow) -> Self {
        AddFavoriteViewModel {
            search_input: add.input.value().to_string(),
            search_results: add.search_results.clone(),
            searching: add.searching,
        }
    }
}
