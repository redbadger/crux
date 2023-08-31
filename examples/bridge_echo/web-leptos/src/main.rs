mod core;

use std::time::Duration;

use leptos::{
    component, create_effect, create_signal, set_timeout, view, IntoView, Scope, SignalGet,
    SignalSet, SignalUpdate,
};

use shared::Event;

#[component]
fn RootComponent(cx: Scope) -> impl IntoView {
    let core = core::new();
    let (view, render) = create_signal(cx, core.view());
    let (event, set_event) = create_signal(cx, Event::Tick);
    let (clock, set_clock) = create_signal(cx, 0u64);

    create_effect(cx, move |_| {
        core::update(&core, event.get(), render);
    });

    // When render happens, send a new tick immediately
    create_effect(cx, move |_| {
        view.get();
        set_timeout(move || set_event.set(Event::Tick), Duration::from_millis(0));
    });

    // Start a new period every second
    create_effect(cx, move |_| {
        clock.get();
        set_timeout(
            move || {
                set_event.set(Event::NewPeriod);
                set_clock.update(|c| *c += 1);
            },
            Duration::from_millis(1000),
        )
    });

    view! {cx,
        <section class="box container has-text-centered m-5">
            <p class="is-size-5">{move || view.get().count}</p>
        </section>
    }
}

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount_to_body(|cx| {
        view! { cx, <RootComponent /> }
    });
}
