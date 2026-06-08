//! The effect handling "lanes" an [`EffectRouter`](super::EffectRouter) can
//! dispatch to.
//!
//! Each route handles a category of effects in a particular way:
//!
//! - [`Serialized`] is the default, bridge-like lane that exchanges bytes with
//!   the shell.
//! - [`Parked`] holds requests under an [`EffectId`](super::EffectId) for a
//!   custom, user-owned FFI carrying opaque or non-serializable data.
//! - [`Buffer`] collects requests for the caller to drain and handle in process.

mod buffer;
mod parked;
mod serialized;

pub use buffer::Buffer;
pub use parked::Parked;
pub use serialized::Serialized;
