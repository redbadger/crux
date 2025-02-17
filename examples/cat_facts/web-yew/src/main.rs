mod core;
mod http;
mod platform;
mod time;

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

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link();
        link.send_message(Message::Event(Event::Get));
        link.send_message(Message::Event(Event::GetPlatform));

        Self { core: core::new() }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let link = ctx.link().clone();
        let callback = Callback::from(move |msg| {
            link.send_message(msg);
        });
        if let Message::Event(event) = msg {
            // TODO: error handling
            let _ = core::update(&self.core, event, &callback);
            false
        } else {
            true
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let view = self.core.view();

        html! {
            <>
                <section class="section title has-text-centered">
                    <p>{&view.platform}</p>
                </section>
                <section class="section container has-text-centered">
                    if let Some(image) = &view.image {
                        <img src={image.href.clone()} style="height: 400px" />
                    }
                </section>
                <section class="section container has-text-centered">
                    <p>{&view.fact}</p>
                </section>
                <div class="buttons container is-centered">
                    <button class="button is-primary is-danger"
                        onclick={link.callback(|_| Message::Event(Event::Clear))}>
                        {"Clear"}
                    </button>
                    <button class="button is-primary is-success"
                        onclick={link.callback(|_| Message::Event(Event::Get))}>
                        {"Get"}
                    </button>
                    <button class="button is-primary is-warning"
                        onclick={link.callback(|_| Message::Event(Event::Fetch))}>
                        {"Fetch"}
                    </button>
                </div>
            </>
        }
    }
}

fn main() {
    yew::Renderer::<RootComponent>::new().render();
}
