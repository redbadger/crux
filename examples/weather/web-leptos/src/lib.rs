//! Leptos CSR web shell for the Weather example.

// The Leptos `#[component]` macro generates `#[must_use]`-triggering helpers.
#![allow(clippy::must_use_candidate)]

mod components;
mod core;

use std::rc::Rc;

use leptos::callback::UnsyncCallback;
use leptos::prelude::*;
use phosphor_leptos::{CLOUD_SUN, WARNING_CIRCLE};

use shared::{
    Event, ViewModel,
    view::{
        active::{ActiveViewModel, favorites::FavoritesViewModel, home::HomeViewModel},
        onboard::OnboardViewModel,
    },
};

use crate::components::{
    DispatchContext,
    common::{Card, ScreenHeader, Spinner, StatusMessage, StatusTone},
    favorites::FavoritesView,
    home::HomeView,
    onboard::OnboardView,
};

// ANCHOR: app
/// The root component: the single point where the core, the view signal,
/// and the dispatcher meet.
///
/// Two pieces of state leave this component:
///
/// - `view: ReadSignal<ViewModel>` — the reactive read-side of the model.
///   Memos below project it into per-stage sub-view-models.
/// - `dispatch: UnsyncCallback<Event>` — an imperative callback handed
///   through context. Events are commands, not state, so they don't live
///   in a signal.
#[component]
pub fn app() -> impl IntoView {
    let core = core::new();
    let (view, set_view) = signal(core.view());

    let dispatch_core = Rc::clone(&core);
    let dispatch = UnsyncCallback::new(move |event: Event| {
        core::update(&dispatch_core, event, set_view);
    });
    provide_context(DispatchContext(dispatch));

    // Fire `Event::Start` once on mount. We defer it to an effect so
    // `provide_context` has finished wiring before any child reads it.
    let start_core = Rc::clone(&core);
    Effect::new(move |_| {
        core::update(&start_core, Event::Start, set_view);
    });

    // ANCHOR: projections
    // Project the top-level view model into per-stage memos. Each screen
    // component takes one of these and reads individual fields via
    // `.read()` or `.with()` inside its own reactive closures.
    //
    // The `_ => Default::default()` branches are never visible: `Show`
    // below only mounts the matching subtree. The Default is load-bearing
    // for types — it lets the Memo produce a concrete `HomeViewModel`
    // regardless of which variant the parent signal is currently in.
    let onboard_vm = Memo::new(move |_| {
        view.with(|v| match v {
            ViewModel::Onboard(m) => m.clone(),
            _ => OnboardViewModel::default(),
        })
    });
    let home_vm = Memo::new(move |_| {
        view.with(|v| match v {
            ViewModel::Active(ActiveViewModel::Home(m)) => m.clone(),
            _ => HomeViewModel::default(),
        })
    });
    let favorites_vm = Memo::new(move |_| {
        view.with(|v| match v {
            ViewModel::Active(ActiveViewModel::Favorites(m)) => m.clone(),
            _ => FavoritesViewModel::default(),
        })
    });
    let failed_message = Memo::new(move |_| {
        view.with(|v| match v {
            ViewModel::Failed { message } => message.clone(),
            _ => String::new(),
        })
    });
    // ANCHOR_END: projections

    view! {
        <div class="max-w-xl mx-auto px-4 py-8">
            <ScreenHeader
                title="Crux Weather"
                subtitle="Rust Core, Rust Shell (Leptos)"
                icon=CLOUD_SUN
            />
            <Show when=move || view.with(|v| matches!(v, ViewModel::Loading))>
                <Card>
                    <Spinner message="Loading..." />
                </Card>
            </Show>
            <Show when=move || view.with(|v| matches!(v, ViewModel::Onboard(_)))>
                <OnboardView vm=onboard_vm />
            </Show>
            <Show when=move || view.with(|v| matches!(v, ViewModel::Active(ActiveViewModel::Home(_))))>
                <HomeView vm=home_vm />
            </Show>
            <Show when=move || view.with(|v| matches!(v, ViewModel::Active(ActiveViewModel::Favorites(_))))>
                <FavoritesView vm=favorites_vm />
            </Show>
            <Show when=move || view.with(|v| matches!(v, ViewModel::Failed { .. }))>
                {move || view! {
                    <Card>
                        <StatusMessage
                            icon=WARNING_CIRCLE
                            message=failed_message.get()
                            tone=StatusTone::Error
                        />
                    </Card>
                }}
            </Show>
        </div>
    }
}
// ANCHOR_END: app
