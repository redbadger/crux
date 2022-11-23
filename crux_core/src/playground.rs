use std::{any::Any, collections::HashMap};

pub struct Store<Effect, Event>(HashMap<usize, Command<Effect, Event>>);

pub struct Command<Effect, Event> {
    effect: Effect, // TODO switch to `enum Effect`, so that shell knows what to do
    resolve: Box<dyn Callback<Event>>,
}

impl<Effect, Event> Command<Effect, Event> {
    pub fn new<F, T>(effect: Effect, resolve: F) -> Self
    where
        F: Fn(T) -> Event + 'static,
        Event: 'static,
        T: 'static,
    {
        Self {
            effect,
            resolve: Box::new(resolve.into_callback()),
        }
    }

    pub fn resolve(&self, value: Box<dyn Any>) -> Event {
        self.resolve.call(value)
    }
}

pub trait Callback<Event> {
    fn call(&self, value: Box<dyn Any>) -> Event;
}

struct CallBackFn<T, Event> {
    function: Box<dyn Fn(T) -> Event>,
}

impl<T, Event> Callback<Event> for CallBackFn<T, Event>
where
    T: 'static,
{
    fn call(&self, value: Box<dyn Any>) -> Event {
        match value.downcast::<T>() {
            Ok(response) => (self.function)(*response),
            Err(_e) => panic!("downcast failed!"),
        }
    }
}

trait IntoCallBack<T, Event> {
    fn into_callback(self) -> CallBackFn<T, Event>;
}

impl<F, T, Event> IntoCallBack<T, Event> for F
where
    F: Fn(T) -> Event + 'static,
{
    fn into_callback(self) -> CallBackFn<T, Event> {
        CallBackFn {
            function: Box::new(self),
        }
    }
}

mod app {
    use super::Command;
    use super::{cap_1, cap_2};

    #[derive(Debug, PartialEq, Eq)]
    pub enum AppEvent {
        Get1,
        Get2,
        Cap1(cap_1::Response), // FnOnce(u8) -> AppEvent
        Cap2(cap_2::Response), // FnOnce(u8) -> AppEvent
    }

    #[derive(Copy, Clone)]
    pub enum Effect {
        Capability1(cap_1::Request),
        Capability2(cap_2::Request),
    }

    pub fn update(event: AppEvent) -> Vec<Command<Effect, AppEvent>> {
        let cap1 = cap_1::Capability1::new(Effect::Capability1);
        let cap2 = cap_2::Capability2::new(Effect::Capability2);
        match event {
            AppEvent::Get1 => vec![cap1.get(1, AppEvent::Cap1)],
            AppEvent::Get2 => vec![cap2.get(2, AppEvent::Cap2)],
            AppEvent::Cap1(_) => vec![],
            AppEvent::Cap2(_) => vec![],
        }
    }
}

mod cap_1 {
    use super::Command;

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
            Ev: 'static,
            F: Fn(Response) -> Ev + 'static,
        {
            Command::new((self.effect)(Request(input)), callback)
        }
    }

    #[derive(Copy, Clone)]
    pub struct Request(pub u16);

    #[derive(Debug, PartialEq, Eq)]
    pub struct Response(pub String);

    // Public API of the capability, called by App::update.
}

mod cap_2 {
    use super::Command;

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
            Ev: 'static,
            F: Fn(Response) -> Ev + 'static,
        {
            Command::new((self.effect)(Request(input)), callback)
        }
    }

    #[derive(Copy, Clone)]
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
        let mut store = Store::<Effect, AppEvent>(HashMap::new());

        let mut command1 = app::update(AppEvent::Get1);
        let mut command2 = app::update(AppEvent::Get2);

        let command1 = command1.remove(0);
        let command2 = command2.remove(0);

        let effect1 = command1.effect;
        let effect2 = command2.effect;

        // store continuation
        store.0.insert(1, command1);
        store.0.insert(2, command2);

        // Shell doing side effects
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
        let command1 = store.0.remove(&1).unwrap();
        let command2 = store.0.remove(&2).unwrap();

        // call continuation with Http response
        let event1: AppEvent = command1.resolve(Box::new(cap_1_response) as Box<dyn Any>);
        let event2: AppEvent = command2.resolve(Box::new(cap_2_response) as Box<dyn Any>);

        assert_eq!(event1, AppEvent::Cap1(cap_1::Response("1".to_string())));
        assert_eq!(event2, AppEvent::Cap2(cap_2::Response("2".to_string())));
    }
}
