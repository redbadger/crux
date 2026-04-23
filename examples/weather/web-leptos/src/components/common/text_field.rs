use leptos::callback::{Callable, UnsyncCallback};
use leptos::prelude::*;
use phosphor_leptos::{Icon, IconData};

/// An input field with an optional leading icon.
///
/// `value` is a `Signal<String>` so screens can drive it from the view model
/// directly; `on_input` is invoked on every keystroke.
#[component]
pub fn text_field(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] placeholder: String,
    on_input: UnsyncCallback<String>,
    #[prop(optional)] icon: Option<IconData>,
    #[prop(optional)] autofocus: bool,
) -> impl IntoView {
    let base_input = "w-full rounded-lg border border-slate-200 bg-white \
                      py-2.5 text-sm text-slate-900 placeholder:text-slate-400 \
                      focus:outline-none focus:ring-2 focus:ring-sky-400 focus:border-sky-400";
    let input_class = if icon.is_some() {
        format!("{base_input} pl-10 pr-3")
    } else {
        format!("{base_input} px-3")
    };

    view! {
        <div class="relative">
            {icon.map(|i| view! {
                <span class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 text-slate-400">
                    <Icon icon=i size="18px" />
                </span>
            })}
            <input
                type="text"
                class=input_class
                placeholder=placeholder
                autofocus=autofocus
                prop:value=move || value.get()
                on:input=move |ev| on_input.run(event_target_value(&ev))
            />
        </div>
    }
}
