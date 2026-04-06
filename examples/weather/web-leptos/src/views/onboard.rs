use leptos::prelude::*;

use shared::{
    Event,
    model::OnboardEvent,
    view::onboard::{OnboardStateViewModel, OnboardViewModel},
};

#[component]
pub fn onboard_view(model: OnboardViewModel, set_event: WriteSignal<Event>) -> impl IntoView {
    let (icon, reason_text) = match model.reason {
        shared::model::onboard::OnboardReason::Welcome => (
            "ph ph-key",
            "Welcome! Enter your OpenWeather API key to get started.",
        ),
        shared::model::onboard::OnboardReason::Unauthorized => (
            "ph ph-warning",
            "Your API key was rejected. Please enter a valid key.",
        ),
        shared::model::onboard::OnboardReason::Reset => (
            "ph ph-arrow-counter-clockwise",
            "Enter a new API key.",
        ),
    };

    match model.state {
        OnboardStateViewModel::Input { api_key, can_submit } => {
            view! {
                <div class="card">
                    <p class="section-title">
                        <i class=icon></i>
                        {"Setup"}
                    </p>
                    <p style="color: #6b7280; margin-bottom: 1rem;">{reason_text}</p>
                    <div class="field">
                        <div class="control has-icons-left">
                            <input
                                class="input"
                                type="text"
                                placeholder="Paste your API key here"
                                prop:value=api_key
                                on:input=move |ev| {
                                    set_event.set(Event::Onboard(OnboardEvent::ApiKey(
                                        event_target_value(&ev),
                                    )));
                                }
                            />
                            <span class="icon is-left">
                                <i class="ph ph-key"></i>
                            </span>
                        </div>
                    </div>
                    <button
                        class="button is-primary btn"
                        disabled=move || !can_submit
                        on:click=move |_| set_event.set(Event::Onboard(OnboardEvent::Submit))
                    >
                        <span class="icon"><i class="ph ph-check"></i></span>
                        <span>{"Submit"}</span>
                    </button>
                </div>
            }.into_any()
        }
        OnboardStateViewModel::Saving => {
            view! {
                <div class="card">
                    <div class="status-message">
                        <i class="ph ph-spinner"></i>
                        <p>{"Saving..."}</p>
                    </div>
                </div>
            }.into_any()
        }
    }
}
