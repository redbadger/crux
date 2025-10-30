use serde::Serialize;

use crate::bridge::{FfiFormat, ResolveSerialized};

/// Implemented automatically with the effect macro from `crux_macros`.
/// This is a marker trait to ensure the macro generated traits are present on the effect type.
///
/// You should annotate your type with `#[effect]` to implement this trait.
// used in docs/internals/bridge.md
// ANCHOR: effect
pub trait Effect: Send + 'static {}
// ANCHOR_END: effect

/// Implemented automatically with the effect macro from `crux_macros`.
/// This is used by the [`Bridge`](crate::bridge::Bridge) to serialize effects going across the
/// FFI boundary. If you don't need serialization and FFI, use [`Effect`].
///
/// You should annotate your type with `#[effect(typegen)]` to implement this trait.
// used in docs/internals/bridge.md
// ANCHOR: effect_typegen
pub trait EffectFFI: Effect {
    /// Ffi is an enum with variants corresponding to the Effect variants
    /// but instead of carrying a `Request<Op>` they carry the `Op` directly
    type Ffi: Serialize;

    /// Converts the `Effect` into its FFI counterpart and returns it alongside
    /// a deserializing version of the resolve callback for the request that the
    /// original `Effect` was carrying.
    ///
    /// You should not need to call this method directly. It is called by
    /// the [`Bridge`](crate::bridge::Bridge)
    fn serialize<T: FfiFormat>(self) -> (Self::Ffi, ResolveSerialized<T>);
}
// ANCHOR_END: effect_typegen
