use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::effects::{command::Location, location::LocationResponse};
use crate::events::current::{update as update_current_weather, CurrentWeatherEvent};
use crate::{Effect, Event, Model};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum HomeEvent {
    Show,
    LocationEnabled(bool),
    LocationFetched(Option<LocationResponse>),
}

pub fn update(event: HomeEvent, model: &mut Model) -> Command<Effect, Event> {
    match event {
        HomeEvent::Show => Location::is_location_enabled()
            .then_send(|result| Event::Home(Box::new(HomeEvent::LocationEnabled(result)))),
        HomeEvent::LocationEnabled(enabled) => {
            model.location_enabled = enabled;
            if enabled {
                Location::get_location()
                    .then_send(|result| Event::Home(Box::new(HomeEvent::LocationFetched(result))))
            } else {
                Command::done()
            }
        }
        HomeEvent::LocationFetched(location) => {
            model.last_location = location.clone();
            if let Some(loc) = location {
                update_current_weather(CurrentWeatherEvent::Fetch(loc.lat, loc.lon), model)
            } else {
                Command::done()
            }
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

        // 1. Trigger the Show event
        let event = Event::Home(Box::new(HomeEvent::Show));
        let _ = app.update(event, &mut model, &());

        // 2. Simulate the Location::is_location_enabled effect (enabled = true)
        let event = Event::Home(Box::new(HomeEvent::LocationEnabled(true)));
        let _ = app.update(event, &mut model, &());

        // 3. Simulate the Location::get_location effect (with a test location)
        let test_location = LocationResponse {
            lat: 33.456789,
            lon: -112.037222,
        };
        let event = Event::Home(Box::new(HomeEvent::LocationFetched(Some(
            test_location.clone(),
        ))));
        let mut cmd = app.update(event, &mut model, &());

        // 4. Resolve the weather HTTP effect
        let mut request = cmd.effects().next().unwrap().expect_http();
        assert_eq!(
            &request.operation,
            &HttpRequest::get(WEATHER_URL)
                .query(&CurrentQueryString {
                    lat: test_location.lat.to_string(),
                    lon: test_location.lon.to_string(),
                    units: "metric",
                    appid: API_KEY.clone(),
                })
                .expect("could not serialize query string")
                .build()
        );

        // 5. Resolve the HTTP request with a simulated response from the web API
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(SAMPLE_CURRENT_RESPONSE_JSON.as_bytes())
                    .build(),
            ))
            .unwrap();

        // 6. The next event should be SetWeather
        let actual = cmd.events().next().unwrap();
        if let Event::CurrentWeather(event) = &actual {
            assert!(matches!(**event, CurrentWeatherEvent::SetWeather(_)))
        } else {
            panic!("Expected CurrentWeather event")
        }

        // 7. Send the SetWeather event back to the app
        let mut cmd = app.update(actual, &mut model, &());
        assert_effect!(cmd, Effect::Render(_));
        // Now check the model in detail
        assert_eq!(model.weather_data, *SAMPLE_CURRENT_RESPONSE);
    }
}
