use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::events::current::{update as update_current_weather, CurrentWeatherEvent};
use crate::{Effect, Event, Model};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum HomeEvent {
    Show(f64, f64),
}

pub fn update(event: HomeEvent, model: &mut Model) -> Command<Effect, Event> {
    match event {
        HomeEvent::Show(lat, long) => {
            update_current_weather(CurrentWeatherEvent::Fetch(lat, long), model)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crux_core::{assert_effect, App as _};
    use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};

    use crate::{
        events::current::{CurrentQueryString, API_KEY, WEATHER_URL},
        App, SAMPLE_CURRENT_RESPONSE, SAMPLE_CURRENT_RESPONSE_JSON,
    };

    #[test]
    fn test_show_triggers_set_weather() {
        let app = App;
        let mut model = Model::default();

        let lat_lng = (33.456789, -112.037222);
        let event = Event::Home(Box::new(HomeEvent::Show(lat_lng.0, lat_lng.1)));

        let mut cmd = app.update(event, &mut model, &());

        let mut request = cmd.effects().next().unwrap().expect_http();

        assert_eq!(
            &request.operation,
            &HttpRequest::get(WEATHER_URL)
                .query(&CurrentQueryString {
                    lat: lat_lng.0.to_string(),
                    lon: lat_lng.1.to_string(),
                    appid: API_KEY,
                })
                .expect("could not serialize query string")
                .build()
        );

        // resolve the request with a simulated response from the web API
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(SAMPLE_CURRENT_RESPONSE_JSON.as_bytes())
                    .build(),
            ))
            .unwrap();

        let actual = cmd.events().next().unwrap();
        if let Event::CurrentWeather(event) = &actual {
            assert!(matches!(**event, CurrentWeatherEvent::SetWeather(_)))
        } else {
            panic!("Expected CurrentWeather event")
        }

        // send the `SetWeather` event back to the app
        let mut cmd = app.update(actual, &mut model, &mut ());
        assert_effect!(cmd, Effect::Render(_));
        // Now check the model in detail
        assert_eq!(model.weather_data, *SAMPLE_CURRENT_RESPONSE);
    }
}
