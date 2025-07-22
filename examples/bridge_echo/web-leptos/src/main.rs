mod core;

use std::time::Duration;

use leptos::prelude::*;
use shared::{DataPoint, Event};

#[component]
fn RootComponent() -> impl IntoView {
    let payload: Vec<_> = (1..10)
        .map(|id| DataPoint {
            value: 3.0 * f64::from(id),
            id: 17,
            label: format!("point_{id}"),
            metadata: if id % 2 == 0 {
                Some(format!("meta_{id}"))
            } else {
                None
            },
        })
        .collect();

    let core = core::new();
    let (view, render) = signal(core.view());
    let (event, set_event) = signal(Event::Tick(payload.clone()));
    let (clock, set_clock) = signal(0u64);

    Effect::new(move |_| {
        core::update(&core, event.get(), render);
    });

    // When render happens, send a new tick immediately
    Effect::new({
        move |_| {
            view.get();
            let payload = payload.clone();

            set_timeout(
                move || set_event.set(Event::Tick(payload)),
                Duration::from_millis(0),
            );
        }
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
        );
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
