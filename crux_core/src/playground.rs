use std::{collections::HashMap, marker::PhantomData};

struct Store<T, Event>(HashMap<usize, Box<dyn MakeEvent<T, Event>>>)
where
    T: CapabilityResponse;

pub struct Command<T, Event>
where
    T: CapabilityResponse,
{
    input: Box<dyn CapabilityRequest>, // TODO switch to `enum Effect`, so that shell knows what to do
    output_to_event: Box<dyn MakeEvent<T, Event>>,
}

trait CapabilityRequest {}

trait CapabilityResponse: Sized {}

pub trait Event {}

struct Continuation<F, T, Event>
where
    F: MakeEvent<T, Event>,
    T: CapabilityResponse,
{
    function: F,
    marker: PhantomData<fn() -> (T, Event)>,
}

trait MakeEvent<T, Event>
where
    T: CapabilityResponse,
{
    fn make_event(&self, value: T) -> Event;
}

struct IntoMakeEvent<T> {
    make_event: T,
}

mod app {
    use super::{cap_1, cap_2, CapabilityResponse};
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
    pub fn update<T>(event: AppEvent) -> Vec<Command<T, AppEvent>>
    where
        T: CapabilityResponse,
    {
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
    use super::{CapabilityRequest, CapabilityResponse, Command, Event, MakeEvent};

    pub struct Cap1Request(bool);
    impl CapabilityRequest for Cap1Request {}

    #[derive(Debug, PartialEq, Eq)]
    pub struct Cap1Response(u8);
    impl CapabilityResponse for Cap1Response {}

    type Cap1Callback<Event> = fn(Cap1Response) -> Event;

    // The capability is ~ `async (bool, (u8) -> Event(u8)) -> Event(u8);`
    // ex. (u8) -> Event(u8) = Event::Capability

    impl<R, E> MakeEvent<R, E> for Cap1Callback<E>
    where
        E: Event + Sized,
        R: CapabilityResponse,
    {
        fn make_event(&self, response: R) -> E {
            (self)(response)
        }
    }

    // Public API of the capability, called by App::update.
    pub fn cap_1_get<R, E>(input: bool, callback: Cap1Callback<E>) -> Command<R, E>
    where
        E: Event + 'static,
        R: CapabilityResponse,
    {
        // convert callback into ???

        Command {
            input: Box::new(Cap1Request(input)),
            output_to_event: Box::new(callback),
        }
    }
}

mod cap_2 {
    use super::{CapabilityRequest, CapabilityResponse, Command, Event, MakeEvent};

    pub struct Cap2Request(bool);
    impl CapabilityRequest for Cap2Request {}

    #[derive(Debug, PartialEq, Eq)]
    pub struct Cap2Response(u8);
    impl CapabilityResponse for Cap2Response {}

    type Cap2Callback<Event> = fn(Cap2Response) -> Event;

    // The capability is ~ `async (bool, (u8) -> Event(u8)) -> Event(u8);`
    // ex. (u8) -> Event(u8) = Event::Capability

    impl<R, E> MakeEvent<R, E> for Cap2Callback<E>
    where
        E: Event + Sized,
        R: CapabilityResponse,
    {
        fn make_event(&self, response: R) -> E {
            (self)(response)
        }
    }

    // Public API of the capability, called by App::update.
    pub fn cap_2_get<R, E>(input: bool, callback: Cap2Callback<E>) -> Command<R, E>
    where
        E: Event + 'static,
        R: CapabilityResponse,
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
    use std::collections::HashMap;

    use crate::playground::{
        app::{App, AppEvent},
        Store,
    };

    use super::app;

    #[test]
    fn test_cap_output_to_event() {
        let store = Store::<AppEvent>(HashMap::new());

        let command1 = app::update(AppEvent::Get1);
        let command2 = app::update(AppEvent::Get2);

        // store continuation
        store.0.insert(1, command1.output_to_event);
        store.0.insert(2, command2.output_to_event);

        // fetch continuation
        let continuation1 = store.0.remove(1).unwrap();
        let continuation2 = store.0.remove(2).unwrap();

        let cap_1_response = Cap1Response(8u8);
        let cap_2_response = Cap2Response(8u8);

        // call continuation with Http response
        let event1: AppEvent = continuation1.make_event(cap_1_response);
        let event2: AppEvent = continuation1.make_event(cap_2_response);

        assert_eq!(event1, vec![AppEvent::Cap1(cap_1_response)]);
        assert_eq!(event2, vec![AppEvent::Cap1(cap_2_response)]);
    }
}
