use serde::Serialize;

use crate::bridge::ResolveSerialized;

/// Implemented automatically with the Effect macro from `crux_macros`.
/// This is used by the [`Bridge`](crate::bridge::Bridge) to serialize effects going across the
/// FFI boundary.
// used in docs/internals/bridge.md
// ANCHOR: effect
pub trait Effect: Send + 'static {
    /// Ffi is an enum with variants corresponding to the Effect variants
    /// but instead of carrying a `Request<Op>` they carry the `Op` directly
    type Ffi: Serialize;

    /// Converts the `Effect` into its FFI counterpart and returns it alongside
    /// a deserializing version of the resolve callback for the request that the
    /// original `Effect` was carrying.
    ///
    /// You should not need to call this method directly. It is called by
    /// the [`Bridge`](crate::bridge::Bridge)
    fn serialize(self) -> (Self::Ffi, ResolveSerialized);
}
// ANCHOR_END: effect
