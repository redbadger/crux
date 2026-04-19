use leptos::callback::{Callable, UnsyncCallback};
use leptos::prelude::*;
use phosphor_leptos::{Icon, IconData};

/// Visual treatment for a [`Button`]. Drives background, text colour, and
/// the hover/disabled story.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Danger,
}

impl ButtonVariant {
    fn classes(self) -> &'static str {
        match self {
            Self::Primary => {
                "bg-sky-600 text-white hover:bg-sky-700 focus-visible:ring-sky-300"
            }
            Self::Secondary => {
                "bg-slate-100 text-slate-800 hover:bg-slate-200 focus-visible:ring-slate-300"
            }
            Self::Danger => {
                "bg-red-600 text-white hover:bg-red-700 focus-visible:ring-red-300"
            }
        }
    }
}

/// A labelled action button.
///
/// `on_click` is an `UnsyncCallback<()>` because the Crux dispatcher is
/// already `UnsyncCallback<Event>` — staying in the same family keeps the
/// call-site tidy and avoids `Send` bounds we can't satisfy in WASM.
#[component]
pub fn button(
    #[prop(into)] label: Signal<String>,
    on_click: UnsyncCallback<()>,
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] icon: Option<IconData>,
    #[prop(optional)] full_width: bool,
    #[prop(into, default = true.into())] enabled: Signal<bool>,
) -> impl IntoView {
    let width = if full_width { "w-full" } else { "" };
    let class = format!(
        "inline-flex items-center justify-center gap-2 rounded-lg px-5 py-2.5 \
         text-sm font-semibold transition-colors duration-150 \
         focus-visible:outline-none focus-visible:ring-2 \
         disabled:cursor-not-allowed disabled:opacity-50 {width} {}",
        variant.classes()
    );

    view! {
        <button
            type="button"
            class=class
            disabled=move || !enabled.get()
            on:click=move |_| {
                if enabled.get() {
                    on_click.run(());
                }
            }
        >
            {icon.map(|i| view! { <Icon icon=i size="18px" /> })}
            <span>{move || label.get()}</span>
        </button>
    }
}
