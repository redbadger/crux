use std::rc::Rc;

use futures_util::TryStreamExt;
use leptos::{prelude::*, task};

use shared::{App, Effect, Event, RandomNumber, RandomNumberRequest, ViewModel};

use crate::{http, sse};

pub type Core = Rc<shared::Core<App>>;

pub fn new() -> Core {
    Rc::new(shared::Core::new())
}

pub fn update(core: &Core, event: Event, render: WriteSignal<ViewModel>) {
    for effect in core.process_event(event) {
        process_effect(core, effect, render);
    }
}

pub fn process_effect(core: &Core, effect: Effect, render: WriteSignal<ViewModel>) {
    match effect {
        Effect::Render(_) => {
            render.update(|view| *view = core.view());
        }
        Effect::Http(mut request) => {
            task::spawn_local({
                let core = core.clone();

                async move {
                    let response = http::request(&request.operation).await;

                    for effect in core
                        .resolve(&mut request, response.into())
                        .expect("should resolve")
                    {
                        process_effect(&core, effect, render);
                    }
                }
            });
        }
        Effect::ServerSentEvents(mut request) => {
            task::spawn_local({
                let core = core.clone();

                async move {
                    let sse_request = request.operation.clone();
                    let mut stream = sse::request(&sse_request).await.unwrap();

                    while let Ok(Some(response)) = stream.try_next().await {
                        for effect in core
                            .resolve(&mut request, response)
                            .expect("should resolve")
                        {
                            process_effect(&core, effect, render);
                        }
                    }
                }
            });
        }
        Effect::Random(mut request) => {
            let RandomNumberRequest(min, max) = request.operation;
            #[allow(clippy::cast_precision_loss)]
            let number = js_sys::Math::random() * (max as f64 - min as f64) + min as f64;
            #[allow(clippy::cast_possible_truncation)]
            let number = number.floor() as isize;

            for effect in core
                .resolve(&mut request, RandomNumber(number))
                .expect("should resolve")
            {
                process_effect(core, effect, render);
            }
        }
    }
}
