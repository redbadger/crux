use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::model::onboard::{OnboardModel, OnboardReason, OnboardState};

/// View model for the onboarding screen where the user enters their API key.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct OnboardViewModel {
    /// Why the user is here — drives the copy shown at the top of the screen.
    pub reason: OnboardReason,
    /// Whether the user is still typing or the key is being saved.
    pub state: OnboardStateViewModel,
}

/// The onboarding screen's current state.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum OnboardStateViewModel {
    /// The user is typing their key. `can_submit` is precomputed so the
    /// shell doesn't duplicate the trim/non-empty check.
    Input { api_key: String, can_submit: bool },
    /// The key has been submitted; the core is writing it to the secret
    /// store. Shells typically show a spinner here.
    Saving,
}

/// The [`Default`] is the empty `Input` variant — used as the fallback when
/// the web shell projects `Signal<ViewModel>` into a `Memo<OnboardViewModel>`
/// for a stage that isn't currently onboarding.
// ANCHOR: onboard_default
impl Default for OnboardStateViewModel {
    fn default() -> Self {
        OnboardStateViewModel::Input {
            api_key: String::new(),
            can_submit: false,
        }
    }
}
// ANCHOR_END: onboard_default

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
