mod core;
mod views;

use leptos::prelude::*;

use shared::{Event, ViewModel};
use views::{favorites::FavoritesView, home::HomeView, onboard::OnboardView};

use shared::view::active::ActiveViewModel;

// ANCHOR: content_view
#[component]
fn root_component() -> impl IntoView {
    let core = core::new();
    let (view, render) = signal(core.view());
    let (event, set_event) = signal(Event::Start);

    Effect::new(move |_| {
        core::update(&core, event.get(), render);
    });

    view! {
        <div class="app-container">
            <div class="app-header">
                <h1 class="title is-3">
                    <i class="ph ph-cloud-sun" style="margin-right: 0.5rem;"></i>
                    {"Crux Weather"}
                </h1>
                <p class="subtitle is-6">{"Rust Core, Rust Shell (Leptos)"}</p>
            </div>
            {move || {
                match view.get() {
                    ViewModel::Loading => {
                        view! {
                            <div class="card">
                                <div class="status-message">
                                    <i class="ph ph-spinner"></i>
                                    <p>{"Loading..."}</p>
                                </div>
                            </div>
                        }.into_any()
                    }
                    ViewModel::Onboard(onboard) => {
                        view! { <OnboardView model=onboard set_event=set_event /> }.into_any()
                    }
                    ViewModel::Active(active) => match active {
                        ActiveViewModel::Home(home) => {
                            view! { <HomeView model=home set_event=set_event /> }.into_any()
                        }
                        ActiveViewModel::Favorites(favorites) => {
                            view! { <FavoritesView model=favorites set_event=set_event /> }.into_any()
                        }
                    }
                    ViewModel::Failed { message } => {
                        view! {
                            <div class="card">
                                <div class="status-message">
                                    <i class="ph ph-warning-circle" style="color: #ef4444;"></i>
                                    <p style="color: #ef4444;">{message}</p>
                                </div>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}
// ANCHOR_END: content_view

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount::mount_to_body(|| {
        view! { <RootComponent /> }
    });
}
