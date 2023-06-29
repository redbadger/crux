use std::rc::Rc;

use yew::prelude::*;

use shared::{Capabilities, Core, Effect, Event, Hello};

#[derive(Default)]
struct RootComponent {
    core: Rc<Core<Effect, Hello>>,
}

enum Task {
    Event(Event),
    Effect(Effect),
}

impl Component for RootComponent {
    type Message = Task;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            core: Rc::new(Core::new::<Capabilities>()),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let link = ctx.link();
        let core = &self.core;

        match msg {
            Task::Event(event) => {
                let effects = core.process_event(event);
                link.send_message_batch(effects.into_iter().map(Task::Effect).collect());
            }
            Task::Effect(effect) => match effect {
                Effect::Render(_) => return true,
            },
        };

        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let view = self.core.view();

        html! {
            <>
                <section class="box container has-text-centered m-5">
                    <p class="is-size-5">{&view.count}</p>
                    <div class="buttons section is-centered">
                        <button class="button is-primary is-danger"
                            onclick={link.callback(|_| Task::Event(Event::Reset))}>
                            {"Reset"}
                        </button>
                        <button class="button is-primary is-success"
                            onclick={link.callback(|_| Task::Event(Event::Increment))}>
                            {"Increment"}
                        </button>
                        <button class="button is-primary is-warning"
                            onclick={link.callback(|_| Task::Event(Event::Decrement))}>
                            {"Decrement"}
                        </button>
                    </div>
                </section>
            </>
        }
    }
}

fn main() {
    yew::Renderer::<RootComponent>::new().render();
}
