use std::{any::Any, collections::HashMap, marker::PhantomData};

pub struct Store<Event>(HashMap<usize, Box<dyn MakeEvent<Event>>>);

pub struct Command<Effect, Event> {
    input: Effect, // TODO switch to `enum Effect`, so that shell knows what to do
    pub output_to_event: Box<dyn MakeEvent<Event>>,
}

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
            Err(_e) => panic!("Invalid type!"),
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
        Cap1(cap_1::Response), // FnOnce(u8) -> AppEvent
        Cap2(cap_2::Response), // FnOnce(u8) -> AppEvent
    }
    impl Event for AppEvent {}

    pub enum Effect {
        Capability1(cap_1::Request),
        Capability2(cap_2::Request),
    }

    // App::update
    pub fn update(event: AppEvent) -> Vec<Command<Effect, AppEvent>> {
        let cap1 = cap_1::Capability1::new(Effect::Capability1);
        let cap2 = cap_2::Capability2::new(Effect::Capability2);
        match event {
            AppEvent::Get1 => vec![cap1.get(1, AppEvent::Cap1)],
            AppEvent::Get2 => vec![cap2.get(2, AppEvent::Cap2)],
            AppEvent::Cap1(_) => vec![],
            AppEvent::Cap2(_) => vec![],
        }
        // eventually requests capability by calling

        // and wants AppEvent::Capability(u8) back
    }
}

mod cap_1 {
    use super::{Command, Event, IntoEventMaker};

    pub struct Capability1<MakeEffect, Ef>
    where
        MakeEffect: Fn(Request) -> Ef,
    {
        effect: MakeEffect,
    }

    impl<MakeEffect, Ef> Capability1<MakeEffect, Ef>
    where
        MakeEffect: Fn(Request) -> Ef,
    {
        pub fn new(effect: MakeEffect) -> Self {
            Self { effect }
        }

        pub fn get<Ev, F>(&self, input: u16, callback: F) -> Command<Ef, Ev>
        where
            Ev: Event + 'static,
            F: Fn(Response) -> Ev + 'static,
        {
            Command {
                input: (self.effect)(Request(input)),
                output_to_event: Box::new(callback.into_event_maker()),
            }
        }
    }

    pub struct Request(pub u16);

    #[derive(Debug, PartialEq, Eq)]
    pub struct Response(pub String);

    // Public API of the capability, called by App::update.
}

mod cap_2 {
    use super::{Command, Event, IntoEventMaker};

    pub struct Capability2<MakeEffect, Ef>
    where
        MakeEffect: Fn(Request) -> Ef,
    {
        effect: MakeEffect,
    }

    impl<MakeEffect, Ef> Capability2<MakeEffect, Ef>
    where
        MakeEffect: Fn(Request) -> Ef,
    {
        pub fn new(effect: MakeEffect) -> Self {
            Self { effect }
        }

        pub fn get<Ev, F>(&self, input: u8, callback: F) -> Command<Ef, Ev>
        where
            Ev: Event + 'static,
            F: Fn(Response) -> Ev + 'static,
        {
            Command {
                input: (self.effect)(Request(input)),
                output_to_event: Box::new(callback.into_event_maker()),
            }
        }
    }

    pub struct Request(pub u8);

    #[derive(Debug, PartialEq, Eq)]
    pub struct Response(pub String);

    // Public API of the capability, called by App::update.
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::collections::HashMap;

    use super::{cap_1, cap_2};

    use super::app::{self, AppEvent, Effect};
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

        // Shell doing side effects

        let effect1 = command1.input;
        let effect2 = command2.input;

        let cap_1_response = match effect1 {
            Effect::Capability1(cap_1::Request(input)) => cap_1::Response(input.to_string()),
            _ => panic!(),
        };
        let cap_2_response = match effect2 {
            Effect::Capability2(cap_2::Request(input)) => cap_2::Response(input.to_string()),
            _ => panic!(),
        };

        // Core continuing

        // fetch continuation
        let continuation1 = store.0.remove(&1).unwrap();
        let continuation2 = store.0.remove(&2).unwrap();

        // call continuation with Http response
        let event1: AppEvent = continuation1.make_event(Box::new(cap_1_response) as Box<dyn Any>);
        let event2: AppEvent = continuation2.make_event(Box::new(cap_2_response) as Box<dyn Any>);

        assert_eq!(event1, AppEvent::Cap1(cap_1::Response("1".to_string())));
        assert_eq!(event2, AppEvent::Cap2(cap_2::Response("2".to_string())));
    }
}
