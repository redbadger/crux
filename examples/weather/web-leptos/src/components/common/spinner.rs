use leptos::prelude::*;

/// A centred loading indicator with an optional caption.
///
/// Uses Tailwind's `animate-spin` on a bordered disc rather than a phosphor
/// icon — the border trick gives a smoother spin with no flicker.
#[component]
pub fn spinner(#[prop(into, default = String::new())] message: String) -> impl IntoView {
    view! {
        <div class="flex flex-col items-center gap-3 py-8" role="status">
            <div class="h-8 w-8 rounded-full border-2 border-slate-200 border-t-sky-600 motion-safe:animate-spin"></div>
            {(!message.is_empty()).then(|| view! {
                <p class="text-slate-500 text-sm">{message}</p>
            })}
        </div>
    }
}
