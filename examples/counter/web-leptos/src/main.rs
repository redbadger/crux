mod http;
mod sse;

use std::rc::Rc;

use futures_util::TryStreamExt;
use leptos::{
    component, create_effect, create_signal, spawn_local, view, IntoView, Scope, SignalGet,
    SignalUpdate, WriteSignal,
};
use shared::{App, Capabilities, Core, Effect, Event, ViewModel};

#[component]
fn RootComponent(cx: Scope) -> impl IntoView {
    let core = Rc::new(Core::new::<Capabilities>());
    let (view_model, set_view_model) = create_signal(cx, core.view());
    let (event, set_event) = create_signal(cx, Event::StartWatch);

    create_effect(cx, move |_| {
        let event = event.get();
        log::debug!("event: {:?}", event);
        let effects = core.process_event(event);
        process_effects(effects, &core, set_view_model);
    });

    view! {cx,
        <>
            <section class="section has-text-centered">
                <p class="title">{"Crux Counter Example"}</p>
            </section>
            <section class="section has-text-centered">
                <p class="is-size-5">{"Rust Core, Rust Shell (Leptos)"}</p>
            </section>
            <section class="container has-text-centered">
                <p class="is-size-5">{move || view_model.get().text}</p>
                <div class="buttons section is-centered">
                    <button class="button is-primary is-warning"
                        on:click=move |_| set_event.update(|value| *value = Event::Decrement)
                    >
                        {"Decrement"}
                    </button>
                    <button class="button is-primary is-danger"
                        on:click=move |_| set_event.update(|value| *value = Event::Increment)
                    >
                        {"Increment"}
                    </button>
                </div>
            </section>
        </>
    }
}

fn process_effects(
    effects: Vec<Effect>,
    core: &Rc<Core<Effect, App>>,
    render: WriteSignal<ViewModel>,
) {
    for effect in effects {
        log::debug!("effect: {:?}", effect);
        match effect {
            Effect::Render(_) => {
                render.update(|view_model| *view_model = core.view());
            }
            Effect::Http(mut request) => {
                spawn_local({
                    let core = core.clone();

                    async move {
                        let response = http::request(&request.operation).await.unwrap();
                        let effects = core.resolve(&mut request, response);
                        process_effects(effects, &core, render);
                    }
                });
            }
            Effect::ServerSentEvents(mut request) => {
                spawn_local({
                    let core = core.clone();

                    async move {
                        let mut stream = sse::request(&request.operation).await.unwrap();

                        while let Ok(Some(response)) = stream.try_next().await {
                            let effects = core.resolve(&mut request, response);
                            process_effects(effects, &core, render);
                        }
                    }
                });
            }
        };
    }
}

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount_to_body(|cx| {
        view! { cx, <RootComponent /> }
    });
}
