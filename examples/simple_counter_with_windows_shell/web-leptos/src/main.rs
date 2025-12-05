mod core;

use leptos::prelude::*;
use shared::Event;

#[component]
fn RootComponent() -> impl IntoView {
    let core = core::new();
    let (view, render) = signal(core.view());
    let (event, set_event) = signal(Event::Reset);

    Effect::new(move |_| {
        core::update(&core, event.get(), render);
    });

    view! {
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
    leptos::mount::mount_to_body(|| {
        view! { <RootComponent /> }
    });
}
