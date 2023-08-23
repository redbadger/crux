use gloo_console::log;
use shared::{
    key_value::{KeyValueOperation, KeyValueOutput},
    platform::PlatformResponse,
    time::TimeResponse,
    CatFactCapabilities, CatFacts, Effect, Event,
};
use std::rc::Rc;
use yew::{platform::spawn_local, Callback};

use crate::{http, platform, time};

pub type Core = Rc<shared::Core<Effect, CatFacts>>;

pub enum Message {
    Event(Event),
    Effect(Effect),
}

pub fn new() -> Core {
    Rc::new(shared::Core::new::<CatFactCapabilities>())
}

pub fn update(core: &Core, event: Event, callback: &Callback<Message>) {
    log!(format!("event: {:?}", event));
    for effect in core.process_event(event) {
        process_effect(core, effect, callback);
    }
}

pub fn process_effect(core: &Core, effect: Effect, callback: &Callback<Message>) {
    log!(format!("effect: {:?}", effect));
    match effect {
        render @ Effect::Render(_) => callback.emit(Message::Effect(render)),
        Effect::Http(mut request) => {
            spawn_local({
                let core = core.clone();
                let callback = callback.clone();

                async move {
                    let response = http::request(&request.operation).await.unwrap();

                    for effect in core.resolve(&mut request, response) {
                        process_effect(&core, effect, &callback);
                    }
                }
            });
        }

        Effect::KeyValue(mut request) => {
            let response = match request.operation {
                KeyValueOperation::Read(_) => KeyValueOutput::Read(None),
                KeyValueOperation::Write(_, _) => KeyValueOutput::Write(false),
            };

            for effect in core.resolve(&mut request, response) {
                process_effect(&core, effect, &callback);
            }
        }

        Effect::Platform(mut request) => {
            let response =
                PlatformResponse(platform::get().unwrap_or_else(|_| "Unknown browser".to_string()));

            for effect in core.resolve(&mut request, response) {
                process_effect(&core, effect, &callback);
            }
        }

        Effect::Time(mut request) => {
            let response = TimeResponse(time::get().unwrap());

            for effect in core.resolve(&mut request, response) {
                process_effect(&core, effect, &callback);
            }
        }
    }
}
