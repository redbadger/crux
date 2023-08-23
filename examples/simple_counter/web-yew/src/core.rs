use shared::{Capabilities, Counter, Effect, Event};
use std::rc::Rc;
use yew::Callback;

pub type Core = Rc<shared::Core<Effect, Counter>>;

pub enum Message {
    Event(Event),
    Effect(Effect),
}

pub fn new() -> Core {
    Rc::new(shared::Core::new::<Capabilities>())
}

pub fn update(core: &Core, event: Event, callback: &Callback<Message>) {
    for effect in core.process_event(event) {
        process_effect(core, effect, callback);
    }
}

pub fn process_effect(_core: &Core, effect: Effect, callback: &Callback<Message>) {
    match effect {
        render @ Effect::Render(_) => callback.emit(Message::Effect(render)),
    }
}
