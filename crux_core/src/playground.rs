use std::{collections::HashMap, marker::PhantomData};

struct Store<Event>(HashMap<usize, Box<dyn MakeEvent<Event>>>);

pub struct Command<Event> {
    input: Box<dyn CapabilityRequest>,
    output_to_event: Box<dyn MakeEvent<Event>>,
}

trait CapabilityRequest {}

trait CapabilityResponse {}

pub trait Event {}

struct Continuation<F, CapabilityOutput, Event>
where
    F: MakeEvent<Event>,
{
    function: F,
    marker: PhantomData<fn() -> (CapabilityOutput, Event)>,
}

trait MakeEvent<Event> {
    fn make(&self);
}

struct IntoMakeEvent<T> {
    make_event: T,
}

mod app {
    use super::{cap_1, cap_2, Store};
    use super::{Command, Event};

    #[derive(Debug)]
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
    pub fn update(_event: AppEvent) -> Command<AppEvent> {
        // eventually requests capability by calling
        cap_1::cap_1_get(true, AppEvent::Cap1)
        // and wants AppEvent::Capability(u8) back
    }
}

mod cap_1 {
    use super::{CapabilityRequest, CapabilityResponse, Command, Event, MakeEvent};

    pub struct Cap1Request(bool);
    impl CapabilityRequest for Cap1Request {}

    #[derive(Debug)]
    pub struct Cap1Response(u8);
    impl CapabilityResponse for Cap1Response {}

    type Cap1Callback<Event> = fn(Cap1Response) -> Event;

    // The capability is ~ `async (bool, (u8) -> Event(u8)) -> Event(u8);`
    // ex. (u8) -> Event(u8) = Event::Capability

    impl<E> MakeEvent<E> for Cap1Callback<E>
    where
        E: Event + Sized,
    {
        fn make(&self) {
            todo!()
        }
    }

    // Public API of the capability, called by App::update.
    pub fn cap_1_get<E>(input: bool, callback: Cap1Callback<E>) -> Command<E>
    where
        E: Event + 'static,
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

    #[derive(Debug)]
    pub struct Cap2Response(u8);
    impl CapabilityResponse for Cap2Response {}

    type Cap2Callback<Event> = fn(Cap2Response) -> Event;

    // The capability is ~ `async (bool, (u8) -> Event(u8)) -> Event(u8);`
    // ex. (u8) -> Event(u8) = Event::Capability

    impl<E> MakeEvent<E> for Cap2Callback<E>
    where
        E: Event + Sized,
    {
        fn make(&self) {
            todo!()
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
        let event1: AppEvent = continuation1.call(cap_1_response);
        let event2: AppEvent = continuation1.call(cap_2_response);

        assert_eq!(event1, AppEvent::Cap1(cap_1_response));
        assert_eq!(event2, AppEvent::Cap1(cap_2_response));
    }
}
