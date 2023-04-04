use serde::Serialize;

use crate::bridge::ResolveBytes;

pub trait Effect: Send + 'static {
    /// Ffi is an enum with variants corresponding to the Effect variants
    /// but instead of carrying a Request<Op> they carry the Op directly
    type Ffi: Serialize;

    fn serialize(self) -> (Self::Ffi, ResolveBytes);
}
