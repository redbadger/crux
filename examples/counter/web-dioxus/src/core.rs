use std::rc::Rc;

use dioxus::{
    prelude::{Signal, UnboundedReceiver},
    signals::WritableExt as _,
};
use futures_util::{StreamExt, TryStreamExt as _};
use tracing::debug;
use wasm_bindgen_futures::spawn_local;

use crate::{http, sse};
use shared::{Counter, Effect, Event, ViewModel};

type Core = Rc<shared::Core<Counter>>;

pub struct CoreService {
    core: Core,
    view: Signal<ViewModel>,
}

impl CoreService {
    pub fn new(view: Signal<ViewModel>) -> Self {
        debug!("initializing core service");
        Self {
            core: Rc::new(shared::Core::new()),
            view,
        }
    }

    pub async fn run(&self, rx: &mut UnboundedReceiver<Event>) {
        let mut view = self.view;
        view.set(self.core.view());
        while let Some(event) = rx.next().await {
            self.update(event, &mut view);
        }
    }

    fn update(&self, event: Event, view: &mut Signal<ViewModel>) {
        debug!("event: {:?}", event);

        for effect in self.core.process_event(event) {
            process_effect(&self.core, effect, view);
        }
    }
}

fn process_effect(core: &Core, effect: Effect, view: &mut Signal<ViewModel>) {
    debug!("effect: {:?}", effect);

    match effect {
        Effect::Render(_) => {
            view.set(core.view());
        }
        Effect::Http(mut request) => {
            spawn_local({
                let mut view = view.to_owned();
                let core = core.clone();

                async move {
                    let response = http::request(&request.operation).await;

                    for effect in core
                        .resolve(&mut request, response.into())
                        .expect("should resolve")
                    {
                        process_effect(&core, effect, &mut view);
                    }
                }
            });
        }

        Effect::ServerSentEvents(mut request) => {
            spawn_local({
                let mut view = view.to_owned();
                let core = core.clone();

                async move {
                    let mut stream = sse::request(&request.operation).await.unwrap();

                    while let Ok(Some(response)) = stream.try_next().await {
                        for effect in core
                            .resolve(&mut request, response)
                            .expect("should resolve")
                        {
                            process_effect(&core, effect, &mut view);
                        }
                    }
                }
            });
        }
    }
}
