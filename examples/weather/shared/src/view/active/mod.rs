pub mod favorites;
pub mod home;

use facet::Facet;
use serde::{Deserialize, Serialize};

pub use favorites::FavoritesViewModel;
pub use home::HomeViewModel;

use crate::model::active::{ActiveModel, Screen};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum ActiveViewModel {
    Home(HomeViewModel),
    Favorites(FavoritesViewModel),
}

impl From<&ActiveModel> for ActiveViewModel {
    fn from(active: &ActiveModel) -> Self {
        match &active.screen {
            Screen::Home(home) => ActiveViewModel::Home(home.into()),
            Screen::Favorites(fav) => ActiveViewModel::Favorites(fav.into()),
        }
    }
}
