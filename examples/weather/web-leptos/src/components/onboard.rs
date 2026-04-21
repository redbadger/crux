use leptos::callback::UnsyncCallback;
use leptos::prelude::*;
use phosphor_leptos::{ARROW_COUNTER_CLOCKWISE, CHECK, KEY, WARNING};

use shared::{
    Event,
    model::{OnboardEvent, onboard::OnboardReason},
    view::onboard::{OnboardStateViewModel, OnboardViewModel},
};

use super::{
    common::{Button, Card, SectionTitle, Spinner, TextField},
    use_dispatch,
};

// ANCHOR: onboard_view
#[component]
pub fn onboard_view(#[prop(into)] vm: Signal<OnboardViewModel>) -> impl IntoView {
    let dispatch = use_dispatch();

    view! {
        {move || match vm.read().state.clone() {
            OnboardStateViewModel::Input { api_key, can_submit } => {
                let (api_key_sig, _) = signal(api_key);
                view! {
                    <Card>
                        {move || {
                            let (icon, reason_text) = reason_copy(vm.read().reason);
                            view! { <SectionTitle icon=icon title=reason_title(vm.read().reason) />
                                <p class="text-slate-500 text-sm mb-4">{reason_text}</p> }
                        }}
                        <div class="mb-4">
                            <TextField
                                value=api_key_sig
                                placeholder="Paste your API key here"
                                icon=KEY
                                on_input=UnsyncCallback::new(move |value| {
                                    dispatch.run(Event::Onboard(OnboardEvent::ApiKey(value)));
                                })
                            />
                        </div>
                        <Button
                            label="Submit"
                            icon=CHECK
                            enabled=Signal::derive(move || can_submit)
                            full_width=true
                            on_click=UnsyncCallback::new(move |()| {
                                dispatch.run(Event::Onboard(OnboardEvent::Submit));
                            })
                        />
                    </Card>
                }.into_any()
            }
            OnboardStateViewModel::Saving => view! {
                <Card>
                    <Spinner message="Saving..." />
                </Card>
            }.into_any(),
        }}
    }
}
// ANCHOR_END: onboard_view

fn reason_title(reason: OnboardReason) -> &'static str {
    match reason {
        OnboardReason::Welcome => "Setup",
        OnboardReason::Unauthorized => "Rejected",
        OnboardReason::Reset => "Reset",
    }
}

fn reason_copy(reason: OnboardReason) -> (phosphor_leptos::IconData, &'static str) {
    match reason {
        OnboardReason::Welcome => (
            KEY,
            "Welcome! Enter your OpenWeather API key to get started.",
        ),
        OnboardReason::Unauthorized => (
            WARNING,
            "Your API key was rejected. Please enter a valid key.",
        ),
        OnboardReason::Reset => (ARROW_COUNTER_CLOCKWISE, "Enter a new API key."),
    }
}
