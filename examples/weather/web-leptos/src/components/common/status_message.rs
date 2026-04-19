use leptos::prelude::*;
use phosphor_leptos::{Icon, IconData};

/// Semantic colour for a [`StatusMessage`].
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum StatusTone {
    #[default]
    Neutral,
    Error,
}

impl StatusTone {
    fn icon_class(self) -> &'static str {
        match self {
            Self::Neutral => "text-slate-400",
            Self::Error => "text-red-500",
        }
    }

    fn text_class(self) -> &'static str {
        match self {
            Self::Neutral => "text-slate-600",
            Self::Error => "text-red-600",
        }
    }
}

/// A centred icon + message used for empty, loading, and failure states
/// inside cards. Saves repeating the same stack of flex-column classes
/// across every screen.
#[component]
pub fn status_message(
    icon: IconData,
    #[prop(into)] message: String,
    #[prop(optional)] tone: StatusTone,
) -> impl IntoView {
    let icon_class = format!("text-3xl {}", tone.icon_class());
    let text_class = format!("text-sm {}", tone.text_class());

    view! {
        <div class="flex flex-col items-center gap-2 py-6 text-center">
            <span class=icon_class>
                <Icon icon=icon size="32px" />
            </span>
            <p class=text_class>{message}</p>
        </div>
    }
}
