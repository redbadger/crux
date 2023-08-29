mod core;

use leptos::{
    component, create_effect, create_signal, view, IntoView, Scope, SignalGet, SignalUpdate,
};
use {{core_name}}::Event;

#[component]
fn RootComponent(cx: Scope) -> impl IntoView {
    let core = core::new();
    let (view, render) = create_signal(cx, core.view());
    let (event, set_event) = create_signal(cx, Event::Reset);

    create_effect(cx, move |_| {
        core::update(&core, event.get(), render);
    });

    view! {cx,
        <section class="box container has-text-centered m-5">
            <p class="is-size-5">{move || view.get().count}</p>
            <div class="buttons section is-centered">
                <button class="button is-primary is-danger"
                    on:click=move |_| set_event.update(|value| *value = Event::Reset)
                >
                    {"Reset"}
                </button>
                <button class="button is-primary is-success"
                    on:click=move |_| set_event.update(|value| *value = Event::Increment)
                >
                    {"Increment"}
                </button>
                <button class="button is-primary is-warning"
                    on:click=move |_| set_event.update(|value| *value = Event::Decrement)
                >
                    {"Decrement"}
                </button>
            </div>
        </section>
    }
}

fn main() {
    leptos::mount_to_body(|cx| {
        view! { cx, <RootComponent /> }
    });
}
