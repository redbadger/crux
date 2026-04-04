use crux_core::{App, Command};

use crate::{
    effects::Effect,
    model::{Event, Model},
    view::ViewModel,
};

#[derive(Default)]
pub struct Weather;

impl App for Weather {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    // ANCHOR: update
    fn update(&self, event: Self::Event, model: &mut Self::Model) -> Command<Effect, Event> {
        model.update(event)
    }
    // ANCHOR_END: update

    // ANCHOR: view
    fn view(&self, model: &Model) -> ViewModel {
        model.into()
    }
    // ANCHOR_END: view
}

#[cfg(test)]
mod tests {
    use crux_core::App as _;

    use crate::{
        effects::secret,
        model::initializing::InitializingModel,
        view::WorkflowViewModel,
    };

    use super::*;

    #[test]
    fn test_start_fetches_secret() {
        let app = Weather;
        let mut model = Model::default();

        let mut cmd = app.update(Event::Start, &mut model);

        assert!(matches!(model, Model::Initializing(_)));
        let request = cmd.expect_one_effect().expect_secret();
        assert_eq!(
            request.operation,
            secret::SecretRequest::Fetch(secret::API_KEY_NAME.to_string())
        );
    }

    #[test]
    fn test_view_loading() {
        let app = Weather;

        let vm = app.view(&Model::Uninitialized);
        assert!(matches!(vm.workflow, WorkflowViewModel::Loading));

        let vm = app.view(&Model::Initializing(InitializingModel));
        assert!(matches!(vm.workflow, WorkflowViewModel::Loading));
    }
}
