use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::model::onboard::{OnboardModel, OnboardReason, OnboardState};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OnboardViewModel {
    pub reason: OnboardReason,
    pub state: OnboardStateViewModel,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum OnboardStateViewModel {
    Input { api_key: String, can_submit: bool },
    Saving,
}

impl From<&OnboardModel> for OnboardViewModel {
    fn from(onboard: &OnboardModel) -> Self {
        let state = match &onboard.state {
            OnboardState::Input { api_key } => OnboardStateViewModel::Input {
                api_key: api_key.clone(),
                can_submit: onboard.can_submit(),
            },
            OnboardState::Saving { .. } => OnboardStateViewModel::Saving,
        };
        OnboardViewModel {
            reason: onboard.reason,
            state,
        }
    }
}
