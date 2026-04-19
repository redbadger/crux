//! The view model — the shape the shell renders.
//!
//! [`ViewModel`] is produced by [`Model`]'s `view` method and mirrors the
//! top-level lifecycle stages. Each stage has a dedicated sub-view-model
//! ([`OnboardViewModel`], [`ActiveViewModel`]) that flattens the model into
//! exactly what the UI needs, so shells don't have to understand the
//! model's internal state machines.

pub mod active;
pub mod onboard;

use facet::Facet;
use serde::{Deserialize, Serialize};

pub use active::ActiveViewModel;
pub use onboard::OnboardViewModel;

use crate::model::Model;

// ANCHOR: view_model
/// The top-level view model, one variant per lifecycle stage.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum ViewModel {
    /// The app is starting up or initialising; shells typically show a
    /// spinner or splash screen.
    Loading,
    /// The app is waiting for the user to enter their API key.
    Onboard(OnboardViewModel),
    /// The app is fully initialised and running.
    Active(ActiveViewModel),
    /// An unrecoverable error occurred; `message` is a user-facing string.
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
