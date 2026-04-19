//! Shared presentational components for the Weather shell.
//!
//! These are plain Tailwind-styled Leptos components with no awareness of
//! Crux: they take labels, signals, and callbacks, and render HTML. Screens
//! in `components/{home,favorites,onboard}.rs` compose them to build the UI.

mod button;
mod card;
mod icon_button;
mod modal;
mod screen_header;
mod section_title;
mod spinner;
mod status_message;
mod text_field;

pub use button::{Button, ButtonVariant};
pub use card::Card;
pub use icon_button::{IconButton, IconButtonVariant};
pub use modal::Modal;
pub use screen_header::ScreenHeader;
pub use section_title::SectionTitle;
pub use spinner::Spinner;
pub use status_message::{StatusMessage, StatusTone};
pub use text_field::TextField;
