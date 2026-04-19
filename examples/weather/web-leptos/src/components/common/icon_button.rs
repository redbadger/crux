use leptos::callback::{Callable, UnsyncCallback};
use leptos::prelude::*;
use phosphor_leptos::{Icon, IconData};

/// Colour treatment for an [`IconButton`].
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum IconButtonVariant {
    #[default]
    Default,
    Danger,
}

impl IconButtonVariant {
    fn classes(self) -> &'static str {
        match self {
            Self::Default => "text-slate-600 hover:bg-slate-100 focus-visible:ring-slate-300",
            Self::Danger => "text-red-600 hover:bg-red-50 focus-visible:ring-red-300",
        }
    }
}

/// A small square button with a single icon and no label — list-row actions,
/// close buttons, and the like.
#[component]
pub fn icon_button(
    icon: IconData,
    on_click: UnsyncCallback<()>,
    #[prop(optional)] variant: IconButtonVariant,
    #[prop(into, default = String::new())] aria_label: String,
) -> impl IntoView {
    let class = format!(
        "inline-flex items-center justify-center h-9 w-9 rounded-lg \
         transition-colors duration-150 focus-visible:outline-none \
         focus-visible:ring-2 {}",
        variant.classes()
    );

    view! {
        <button
            type="button"
            class=class
            aria-label=aria_label
            on:click=move |_| on_click.run(())
        >
            <Icon icon=icon size="18px" />
        </button>
    }
}
