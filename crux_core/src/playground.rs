use std::{any::Any, collections::HashMap, marker::PhantomData};

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

struct EventMaker<T, Event> {
    function: Box<dyn Fn(T) -> Event>,
    marker: PhantomData<T>,
}

impl<T, Event> MakeEvent<Event> for EventMaker<T, Event>
where
    T: 'static,
{
    fn make_event(&self, value: Box<dyn Any>) -> Event {
        match value.downcast::<T>() {
            Ok(response) => (self.function)(*response),
            Err(_e) => panic!("Expected a Cap1Response to be returned!"),
        }
    }
}

trait IntoEventMaker<T, Event> {
    fn into_event_maker(self) -> EventMaker<T, Event>;
}

impl<F, T, Event> IntoEventMaker<T, Event> for F
where
    F: Fn(T) -> Event + 'static,
{
    fn into_event_maker(self) -> EventMaker<T, Event> {
        EventMaker {
            function: Box::new(self),
            marker: PhantomData,
        }
    }
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
    use super::{CapabilityRequest, CapabilityResponse, Command, Event, IntoEventMaker};

    pub struct Cap1Request(bool);
    impl CapabilityRequest for Cap1Request {}

    #[derive(Debug, PartialEq, Eq)]
    pub struct Cap1Response(pub u8);
    impl CapabilityResponse for Cap1Response {}

    type Cap1Callback<Event> = fn(Cap1Response) -> Event;

    // Public API of the capability, called by App::update.
    pub fn cap_1_get<E, F>(input: bool, callback: F) -> Command<E>
    where
        E: Event + 'static,
        F: Fn(Cap1Response) -> E + 'static,
    {
        Command {
            input: Box::new(Cap1Request(input)),
            output_to_event: Box::new(callback.into_event_maker()),
        }
    }
}

mod cap_2 {
    use super::{CapabilityRequest, CapabilityResponse, Command, Event, IntoEventMaker};

    pub struct Cap2Request(bool);
    impl CapabilityRequest for Cap2Request {}

    #[derive(Debug, PartialEq, Eq)]
    pub struct Cap2Response(pub u8);
    impl CapabilityResponse for Cap2Response {}

    // The capability is ~ `async (bool, (u8) -> Event(u8)) -> Event(u8);`
    // ex. (u8) -> Event(u8) = Event::Capability

    // Public API of the capability, called by App::update.
    pub fn cap_2_get<E, F>(input: bool, callback: F) -> Command<E>
    where
        E: Event + 'static,
        F: Fn(Cap2Response) -> E + 'static + Sized,
    {
        Command {
            input: Box::new(Cap2Request(input)),
            output_to_event: Box::new(callback.into_event_maker()),
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
