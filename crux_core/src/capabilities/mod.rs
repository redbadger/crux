pub mod compose;
pub mod render;

/// An empty set of capabilities.
///
/// This should be used to satisfy the current version of the [`App`](crate::App) trait
/// until it is updated to remove the `Capabilities` associated type as part
/// of the completion of the migration to the new [`Command`](crate::Command) based API.
pub struct EmptyCapabilities {}
