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
        view::ViewModel,
    };

    use super::*;

    #[test]
    fn start_fetches_secret_and_favorites() {
        let app = Weather;
        let mut model = Model::default();

        let mut cmd = app.update(Event::Start, &mut model);

        assert!(matches!(model, Model::Initializing(_)));

        let secret_request = cmd.expect_effect().expect_secret();
        assert_eq!(
            secret_request.operation,
            secret::SecretRequest::Fetch(secret::API_KEY_NAME.to_string())
        );

        let kv_request = cmd.expect_one_effect().expect_key_value();
        assert!(matches!(
            kv_request.operation,
            crux_kv::KeyValueOperation::Get { .. }
        ));
    }

    #[test]
    fn view_loading() {
        let app = Weather;

        let vm = app.view(&Model::Uninitialized);
        assert!(matches!(vm, ViewModel::Loading));

        let vm = app.view(&Model::Initializing(InitializingModel::default()));
        assert!(matches!(vm, ViewModel::Loading));
    }
}
