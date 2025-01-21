use std::rc::Rc;

use dioxus::{
    prelude::{Signal, UnboundedReceiver},
    signals::Writable,
};
use futures_util::StreamExt;
use shared::{Counter, Effect, Event, ViewModel};
use tracing::debug;

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
    };
}
