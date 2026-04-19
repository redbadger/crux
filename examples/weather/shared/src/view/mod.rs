pub mod active;
pub mod onboard;

use facet::Facet;
use serde::{Deserialize, Serialize};

pub use active::ActiveViewModel;
pub use onboard::OnboardViewModel;

use crate::model::Model;

// ANCHOR: view_model
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum ViewModel {
    Loading,
    Onboard(OnboardViewModel),
    Active(ActiveViewModel),
    Failed { message: String },
}
// ANCHOR_END: view_model

// ANCHOR: view
impl From<&Model> for ViewModel {
    fn from(model: &Model) -> Self {
        match model {
            Model::Uninitialized | Model::Initializing(_) => ViewModel::Loading,
            Model::Onboard(onboard) => ViewModel::Onboard(onboard.into()),
            Model::Active(active) => ViewModel::Active(active.into()),
            Model::Failed(message) => ViewModel::Failed {
                message: message.clone(),
            },
        }
    }
}
// ANCHOR_END: view
