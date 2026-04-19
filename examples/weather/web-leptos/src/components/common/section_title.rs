use leptos::prelude::*;
use phosphor_leptos::{Icon, IconData};

/// An inline card header — icon + title, used at the top of a [`super::Card`]
/// to label the section. Smaller and denser than [`super::ScreenHeader`].
#[component]
pub fn section_title(icon: IconData, #[prop(into)] title: String) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2 text-slate-700 font-semibold text-lg mb-4">
            <Icon icon=icon size="20px" />
            <span>{title}</span>
        </div>
    }
}
