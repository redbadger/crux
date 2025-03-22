use shared::{Counter, Event, Middleware, Nacre, ShellEffect, ViewModel};
use std::rc::Rc;
use yew::Callback;

pub type Core = Rc<Nacre<Counter>>;

pub enum Message {
    Event(Event),
    #[allow(dead_code)]
    Effect(ShellEffect),
}

pub fn new(cb: Callback<Message>) -> Core {
    let (sender, receiver) = async_std::channel::bounded(1);

    let nacre = Nacre::new(sender);
    let this = Rc::new(nacre);
    let cloned = this.clone();

    async_std::task::spawn_local(async move {
        while let Ok(effects) = receiver.recv().await {
            for effect in effects {
                process_effect(&cloned, effect, &cb);
            }
        }
    });
    this
}

pub fn update(core: &Core, event: Event, callback: &Callback<Message>) {
    for effect in core.process_event(event) {
        process_effect(core, effect.into(), callback);
    }
}

pub fn process_effect(_core: &Core, effect: ShellEffect, callback: &Callback<Message>) {
    match effect {
        render @ ShellEffect::Render(_) => callback.emit(Message::Effect(render)),
    }
}

pub fn view(core: &Core) -> ViewModel {
    core.view()
}
