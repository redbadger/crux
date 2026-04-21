use leptos::prelude::*;
use phosphor_leptos::{Icon, IconData};

/// The big white app header — title, optional subtitle, optional icon.
/// Lives at the top of the page above all cards.
#[component]
pub fn screen_header(
    #[prop(into)] title: String,
    #[prop(into, default = String::new())] subtitle: String,
    #[prop(optional)] icon: Option<IconData>,
) -> impl IntoView {
    view! {
        <header class="text-center text-white mb-6">
            <h1 class="text-3xl font-semibold flex items-center justify-center gap-2">
                {icon.map(|i| view! { <Icon icon=i size="28px" /> })}
                <span>{title}</span>
            </h1>
            {(!subtitle.is_empty()).then(|| view! {
                <p class="text-white/80 text-sm mt-1">{subtitle}</p>
            })}
        </header>
    }
}
