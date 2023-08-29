mod core;

use crate::core::{Core, Message};
use shared::Event;
use yew::prelude::*;

#[derive(Default)]
struct RootComponent {
    core: Core,
}

impl Component for RootComponent {
    type Message = Message;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self { core: core::new() }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let link = ctx.link().clone();
        let callback = Callback::from(move |msg| {
            link.send_message(msg);
        });
        if let Message::Event(event) = msg {
            core::update(&self.core, event, &callback);
            false
        } else {
            true
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let view = self.core.view();

        html! {
            <section class="box container has-text-centered m-5">
                <p class="is-size-5">{&view.count}</p>
                <div class="buttons section is-centered">
                    <button class="button is-primary is-danger"
                        onclick={link.callback(|_| Message::Event(Event::Reset))}>
                        {"Reset"}
                    </button>
                    <button class="button is-primary is-success"
                        onclick={link.callback(|_| Message::Event(Event::Increment))}>
                        {"Increment"}
                    </button>
                    <button class="button is-primary is-warning"
                        onclick={link.callback(|_| Message::Event(Event::Decrement))}>
                        {"Decrement"}
                    </button>
                </div>
            </section>
        }
    }
}

fn main() {
    yew::Renderer::<RootComponent>::new().render();
}
