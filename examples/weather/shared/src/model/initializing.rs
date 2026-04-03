use crux_core::{Command, render::render};

use crate::effects::{Effect, secret::SecretFetchResponse};

use super::{
    ActiveEvent, ActiveModel, Event, InitializingEvent, Model, WeatherEvent,
    configuration::ConfigurationModel,
};

pub fn update(event: InitializingEvent, model: &mut Model) -> Command<Effect, Event> {
    match event {
        InitializingEvent::SecretFetched(response) => match response {
            SecretFetchResponse::Fetched(api_key) => {
                *model = Model::Active(ActiveModel {
                    api_key,
                    ..Default::default()
                });
                Command::event(Event::Active(ActiveEvent::Home(Box::new(
                    WeatherEvent::Show,
                ))))
            }
            SecretFetchResponse::Missing(_) => {
                *model = Model::Configuration(ConfigurationModel::default());
                render()
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use crux_core::App as _;

    use crate::{
        app::Weather,
        effects::secret,
        view::WorkflowViewModel,
    };

    use super::*;

    #[test]
    fn test_secret_missing_shows_configuration() {
        let app = Weather;
        let mut model = Model::Initializing;

        let mut cmd = app.update(
            Event::Initializing(InitializingEvent::SecretFetched(
                SecretFetchResponse::Missing(secret::API_KEY_NAME.to_string()),
            )),
            &mut model,
        );

        assert!(matches!(model, Model::Configuration(_)));
        cmd.expect_one_effect().expect_render();

        let vm = app.view(&model);
        assert!(matches!(vm.workflow, WorkflowViewModel::Configuration { .. }));
    }

    #[test]
    fn test_secret_fetched_transitions_to_active() {
        let app = Weather;
        let mut model = Model::Initializing;

        let mut cmd = app.update(
            Event::Initializing(InitializingEvent::SecretFetched(
                SecretFetchResponse::Fetched("my_key".to_string()),
            )),
            &mut model,
        );

        match &model {
            Model::Active(active) => assert_eq!(active.api_key, "my_key"),
            other => panic!("Expected Active, got {other:?}"),
        }

        // Should emit an event to show the home screen
        let event = cmd.expect_one_event();
        assert!(matches!(
            event,
            Event::Active(ActiveEvent::Home(ref we)) if matches!(**we, WeatherEvent::Show)
        ));
    }
}
