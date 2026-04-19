use leptos::prelude::*;

/// A rounded white panel with a soft shadow. The main layout container for
/// screen content — every screen stacks one or more cards.
#[component]
pub fn card(children: Children, #[prop(optional, into)] class: String) -> impl IntoView {
    let full = format!("bg-white rounded-2xl shadow-lg p-6 {class}");
    view! { <div class=full>{children()}</div> }
}
