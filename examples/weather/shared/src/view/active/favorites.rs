use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    effects::{http::location::GeocodingResponse, location::Location},
    model::active::favorites::{
        FavoritesScreen, FavoritesWorkflow, add::AddFavoriteWorkflow, model::Favorite,
    },
};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct FavoritesViewModel {
    pub favorites: Vec<FavoriteViewModel>,
    pub workflow: Option<FavoritesWorkflowViewModel>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FavoriteViewModel {
    pub name: String,
    pub location: Location,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum FavoritesWorkflowViewModel {
    ConfirmDelete { location: Location },
    Add(AddFavoriteViewModel),
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct AddFavoriteViewModel {
    pub search_input: String,
    pub search_results: Option<Vec<GeocodingResponse>>,
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
