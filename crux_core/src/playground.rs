use std::{any::Any, collections::HashMap};

struct Store<Event>(HashMap<usize, Box<dyn MakeEvent<Event>>>);

pub struct Command<Event> {
    input: Box<dyn CapabilityRequest>, // TODO switch to `enum Effect`, so that shell knows what to do
    pub output_to_event: Box<dyn MakeEvent<Event>>,
}

trait CapabilityRequest {}

trait CapabilityResponse: Any {}

pub trait Event {}

pub trait MakeEvent<Event> {
    fn make_event(&self, value: Box<dyn Any>) -> Event;
}

mod app {
    use super::{cap_1, cap_2};
    use super::{Command, Event};

    #[derive(Debug, PartialEq, Eq)]
    pub enum AppEvent {
        Get1,
        Get2,
        Cap1(cap_1::Cap1Response), // FnOnce(u8) -> AppEvent
        Cap2(cap_2::Cap2Response), // FnOnce(u8) -> AppEvent
    }
    impl Event for AppEvent {}

    enum Effect {
        Capability(cap_1::Cap1Request),
    }

    // App::update
    pub fn update(event: AppEvent) -> Vec<Command<AppEvent>> {
        match event {
            AppEvent::Get1 => vec![cap_1::cap_1_get(true, AppEvent::Cap1)],
            AppEvent::Get2 => vec![cap_2::cap_2_get(true, AppEvent::Cap2)],
            AppEvent::Cap1(_) => vec![],
            AppEvent::Cap2(_) => vec![],
        }
        // eventually requests capability by calling

        // and wants AppEvent::Capability(u8) back
    }
}

mod cap_1 {
    use std::any::Any;

    use super::{CapabilityRequest, CapabilityResponse, Command, Event, MakeEvent};

    pub struct Cap1Request(bool);
    impl CapabilityRequest for Cap1Request {}

    #[derive(Debug, PartialEq, Eq)]
    pub struct Cap1Response(pub u8);
    impl CapabilityResponse for Cap1Response {}

    type Cap1Callback<Event> = fn(Cap1Response) -> Event;

    impl<E> MakeEvent<E> for Cap1Callback<E>
    where
        E: Event + Sized,
    {
        // We need to know specific response type here at the latest
        fn make_event(&self, response: Box<dyn Any>) -> E {
            match response.downcast::<Cap1Response>() {
                Ok(response) => self(*response),
                Err(e) => panic!("Expected a Cap1Response to be returned!"),
            }
        }
    }

    // Public API of the capability, called by App::update.
    pub fn cap_1_get<E>(input: bool, callback: Cap1Callback<E>) -> Command<E>
    where
        E: Event + 'static,
    {
        Command {
            input: Box::new(Cap1Request(input)),
            output_to_event: Box::new(callback),
        }
    }
}

mod cap_2 {
    use super::{CapabilityRequest, CapabilityResponse, Command, Event, MakeEvent};
    use std::any::Any;

    pub struct Cap2Request(bool);
    impl CapabilityRequest for Cap2Request {}

    #[derive(Debug, PartialEq, Eq)]
    pub struct Cap2Response(pub u8);
    impl CapabilityResponse for Cap2Response {}

    type Cap2Callback<Event> = fn(Cap2Response) -> Event;

    // The capability is ~ `async (bool, (u8) -> Event(u8)) -> Event(u8);`
    // ex. (u8) -> Event(u8) = Event::Capability

    impl<E> MakeEvent<E> for Cap2Callback<E>
    where
        E: Event + Sized,
    {
        // We need to know specific response type here at the latest
        fn make_event(&self, response: Box<dyn Any>) -> E {
            match response.downcast::<Cap2Response>() {
                Ok(response) => self(*response),
                Err(_) => panic!("Expected a Cap2Response to be returned!"),
            }
        }
    }

    // Public API of the capability, called by App::update.
    pub fn cap_2_get<E>(input: bool, callback: Cap2Callback<E>) -> Command<E>
    where
        E: Event + 'static,
    {
        // convert callback into ???

        Command {
            input: Box::new(Cap2Request(input)),
            output_to_event: Box::new(callback),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::collections::HashMap;

    use super::cap_1::Cap1Response;
    use super::cap_2::Cap2Response;

    use super::app::{self, AppEvent};
    use super::Store;

    #[test]
    fn test_cap_output_to_event() {
        let mut store = Store::<AppEvent>(HashMap::new());

        let mut command1 = app::update(AppEvent::Get1);
        let mut command2 = app::update(AppEvent::Get2);

        let command1 = command1.remove(0);
        let command2 = command2.remove(0);

        // store continuation
        store.0.insert(1, command1.output_to_event);
        store.0.insert(2, command2.output_to_event);

        // fetch continuation
        let continuation1 = store.0.remove(&1).unwrap();
        let continuation2 = store.0.remove(&2).unwrap();

        let cap_1_response = Cap1Response(1u8);
        let cap_2_response = Cap2Response(2u8);

        // call continuation with Http response
        let event1: AppEvent = continuation1.make_event(Box::new(cap_1_response) as Box<dyn Any>);
        let event2: AppEvent = continuation2.make_event(Box::new(cap_2_response) as Box<dyn Any>);

        assert_eq!(event1, AppEvent::Cap1(Cap1Response(1u8)));
        assert_eq!(event2, AppEvent::Cap2(Cap2Response(2u8)));
    }
}
