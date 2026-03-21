mod core;
mod kv;
mod pubsub;
mod time;

use leptos::prelude::*;

use shared::Event;

#[component]
fn root_component() -> impl IntoView {
    let core = core::new();
    let (view, render) = signal(core.view());

    core::update(&core, Event::Open, render);

    let core_for_edit = core.clone();
    let on_edit = move |event: Event| {
        core::update(&core_for_edit, event, render);
    };

    view! {
        <>
            <section class="section has-text-centered">
                <p class="title">{"Crux Notes Example"}</p>
                <p class="is-size-5">{"Rust Core, Rust Shell (Leptos)"}</p>
            </section>
            <section class="container">
                <div class="box">
                    <textarea
                        class="textarea"
                        rows="20"
                        prop:value=move || view.get().text
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            on_edit(Event::Replace(0, view.get().text.len(), value));
                        }
                    />
                </div>
            </section>
        </>
    }
}

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount::mount_to_body(|| {
        view! { <RootComponent /> }
    });
}
