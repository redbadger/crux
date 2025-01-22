mod core;

use std::time::Duration;

use leptos::prelude::*;
use shared::Event;

#[component]
fn RootComponent() -> impl IntoView {
    let core = core::new();
    let (view, render) = signal(core.view());
    let (event, set_event) = signal(Event::Tick);
    let (clock, set_clock) = signal(0u64);

    Effect::new(move |_| {
        core::update(&core, event.get(), render);
    });

    // When render happens, send a new tick immediately
    Effect::new(move |_| {
        view.get();
        set_timeout(move || set_event.set(Event::Tick), Duration::from_millis(0));
    });

    // Start a new period every second
    Effect::new(move |_| {
        clock.get();
        set_timeout(
            move || {
                set_event.set(Event::NewPeriod);
                set_clock.update(|c| *c += 1);
            },
            Duration::from_millis(1000),
        )
    });

    view! {
        <section class="box container has-text-centered m-5">
            <p class="is-size-5">{move || view.get().count}</p>
        </section>
    }
}

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount::mount_to_body(|| {
        view! { <RootComponent /> }
    });
}
