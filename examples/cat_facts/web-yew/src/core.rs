use gloo_console::log;
use shared::{platform::PlatformResponse, time::TimeResponse, CatFacts, Effect, Event};
use std::rc::Rc;
use yew::{platform::spawn_local, Callback};

use crate::{http, platform, time};

pub type Core = Rc<shared::Core<CatFacts>>;

pub enum Message {
    Event(Event),
    #[allow(dead_code)]
    Effect(Effect),
}

pub fn new() -> Core {
    Rc::new(shared::Core::new())
}

pub fn update(core: &Core, event: Event, callback: &Callback<Message>) -> anyhow::Result<()> {
    log!(format!("event: {:?}", event));
    for effect in core.process_event(event) {
        process_effect(core, effect, callback)?;
    }

    Ok(())
}

pub fn process_effect(
    core: &Core,
    effect: Effect,
    callback: &Callback<Message>,
) -> anyhow::Result<()> {
    log!(format!("effect: {:?}", effect));
    match effect {
        render @ Effect::Render(_) => callback.emit(Message::Effect(render)),
        Effect::Http(mut request) => {
            spawn_local({
                let core = core.clone();
                let callback = callback.clone();

                async move {
                    let response = http::request(&request.operation).await;

                    for effect in core
                        .resolve(&mut request, response.into())
                        .expect("effect should resolve")
                    {
                        process_effect(&core, effect, &callback).expect("effect should process");
                    }
                }
            });
        }

        Effect::KeyValue(..) => {}

        Effect::Platform(mut request) => {
            let response =
                PlatformResponse(platform::get().unwrap_or_else(|_| "Unknown browser".to_string()));

            for effect in core.resolve(&mut request, response)? {
                process_effect(core, effect, callback)?;
            }
        }

        Effect::Time(mut request) => {
            let response = TimeResponse::Now {
                instant: time::get(),
            };

            for effect in core.resolve(&mut request, response)? {
                process_effect(core, effect, callback)?;
            }
        }
    }

    Ok(())
}
