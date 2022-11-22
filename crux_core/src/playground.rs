use std::{collections::HashMap, marker::PhantomData};

struct Store<Event>(HashMap<String, Box<dyn MakeEvent<Event>>>);

pub struct Command<Event> {
    input: Box<dyn CapabilityInput>,
    output_to_event: Box<dyn MakeEvent<Event>>,
}

trait CapabilityInput {}

trait CapabilityOutput {}

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
    use super::capability;
    use super::{Command, Event};

    enum AppEvent {
        Capability(capability::Output), // FnOnce(u8) -> AppEvent
        SomeOther,
        Whatever,
    }
    impl Event for AppEvent {}

    enum Effect {
        Capability(capability::Input),
    }

    // App::update
    fn update(_event: AppEvent) -> Command<AppEvent> {
        // eventually requests capability by calling
        capability::capability(true, AppEvent::Capability)
        // and wants AppEvent::Capability(u8) back
    }
}

mod capability {
    use super::{CapabilityInput, CapabilityOutput, Command, Event, MakeEvent};

    pub struct Input(bool);
    impl CapabilityInput for Input {}

    pub struct Output(u8);
    impl CapabilityOutput for Output {}

    type Callback<Event> = fn(Output) -> Event;

    // The capability is ~ `async (bool, (u8) -> Event(u8)) -> Event(u8);`
    // ex. (u8) -> Event(u8) = Event::Capability

    impl<E> MakeEvent<E> for Callback<E>
    where
        E: Event + Sized,
    {
        fn make(&self) {
            todo!()
        }
    }

    // Public API of the capability, called by App::update.
    pub fn capability<E>(input: bool, callback: Callback<E>) -> Command<E>
    where
        E: Event + 'static,
    {
        // convert callback into ???

        let output_to_event = Box::new(callback);

        Command {
            input: Box::new(Input(input)),
            output_to_event,
        }
    }
}
