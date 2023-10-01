mod core;

use std::time::Duration;

use leptos::{
    component, create_effect, create_signal, set_timeout, view, IntoView, Scope, SignalGet,
    SignalGetUntracked, SignalSet, SignalUpdate,
};

use leptos_chart::{Cartesian, LineChart, Series};
use shared::Event;

const PERIOD: u64 = 1000;

#[component]
fn RootComponent(cx: Scope) -> impl IntoView {
    let core = core::new();
    let (view, render) = create_signal(cx, core.view());
    let (event, set_event) = create_signal(cx, Event::Tick);
    let (clock, set_clock) = create_signal(cx, 0u64);

    create_effect(cx, move |_| {
        core::update(&core, event.get(), render);
    });

    // When render happens, send a new tick immediately, if running
    create_effect(cx, move |_| {
        if view.get().running {
            set_timeout(move || set_event.set(Event::Tick), Duration::from_millis(0));
        }
    });

    // Start a new period every second
    create_effect(cx, move |_| {
        clock.get();
        if view.get_untracked().running {
            set_timeout(
                move || {
                    set_event.set(Event::NewPeriod);
                    set_clock.update(|c| *c += 1);
                },
                Duration::from_millis(PERIOD),
            )
        }
    });

    let chart = move || {
        clock.get();

        let log: Vec<_> = view.get_untracked().log;

        let count = log.len() as i64;
        // let max = *log.iter().max().unwrap_or(&0);

        let (x, y) = if count > 0 {
            (
                (1..=count).collect::<Vec<_>>(),
                log.iter().map(|i| *i as f64).collect::<Vec<_>>(),
            )
        } else {
            (vec![0, 1], vec![0.0, 1.0])
        };

        log::debug!("Chart: {:?}, {:?}", x, y);

        Cartesian::new(Series::from(x), Series::from(y)).set_view(420, 380, 3, 100, 100, 20)
    };

    view! {cx,
        <section class="box container has-text-centered my-5 mx-auto">
            <div class="is-flex flex-direction-row is-justify-content-center is-align-items-flex-end">
                <p class="is-size-2 has-text-black has-text-right" style="min-width: 5em">{move || format!("{:.2}", view.get().rate)}</p>
                <p class="is-medium has-text-grey pb-3 has-text-left mx-3" style="min-width: 8em">{move || view.get().count}</p>
                <p class="pb-3 has-text-left">
                    <button class="button"
                    class=("is-danger", move || view.get().running)
                    class=("is-success", move || !view.get().running)
                    on:click=move |_| {
                        let evt = if view.get().running { Event::Stop } else { Event::Start(PERIOD as usize) };
                        set_event.update(|v| *v = evt);
                        set_clock.update(|v| *v += 1);
                    }
                    >
                        {move || if view.get().running { "Stop" } else { "Start" }}
                    </button>
                </p>
            </div>
            {move || {
                let chart = chart();

                view! {cx, <LineChart chart=chart />}
            }}
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
