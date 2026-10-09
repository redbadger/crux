//! spike: the Chrome gateway between the Circuit Playground Bluefruit and the internet.

mod ble;
mod clock;
mod core;
mod http;
mod sse;

use leptos::prelude::*;

use gateway_core::Event;

#[component]
fn root_component() -> impl IntoView {
    let core = core::new();
    let (view, render) = signal(core.view());

    let connect = move |_| core::update(&core, Event::Connect, render);

    view! {
        <main>
            <h1>"CPB gateway"</h1>
            <p class="muted">
                "Forwards the Circuit Playground Bluefruit's HTTP requests and SSE streams \
                 from Bluetooth to the internet."
            </p>
            <p class="status">
                <button on:click=connect disabled=move || !view.get().can_connect>
                    "Connect"
                </button>
                <span>{move || view.get().status}</span>
            </p>
            <p class="stats">
                {move || {
                    let view = view.get();
                    let stats = view.stats;
                    format!(
                        "{} requests · {} responses · {} SSE chunks · {} open streams · {} B in · {} B out",
                        stats.requests,
                        stats.responses,
                        stats.sse_chunks,
                        view.open_streams,
                        stats.bytes_in,
                        stats.bytes_out,
                    )
                }}
            </p>
            <ul class="log">
                {move || {
                    view.get()
                        .log
                        .into_iter()
                        .map(|line| {
                            view! {
                                <li>
                                    <span class="time">{clock::time_of(line.seq)}</span>
                                    " "
                                    {line.text}
                                </li>
                            }
                        })
                        .collect_view()
                }}
            </ul>
        </main>
    }
}

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount::mount_to_body(|| {
        view! { <RootComponent /> }
    });
}
