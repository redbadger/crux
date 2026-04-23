use leptos::prelude::*;

/// A fixed overlay with a centred content slot. Used for the delete
/// confirmation dialog; no close-on-backdrop-click — the caller wires up
/// explicit cancel/confirm buttons inside `children`.
#[component]
pub fn modal(children: Children) -> impl IntoView {
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center px-4">
            <div class="absolute inset-0 bg-slate-900/50"></div>
            <div class="relative z-10 w-full max-w-sm">
                {children()}
            </div>
        </div>
    }
}
