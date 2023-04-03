use serde::Serialize;

use crate::steps::Resolve;

pub trait Effect: Send + 'static {
    /// Ffi is an enum with variants corresponsing to the Effect variants
    /// but instead of carrying a Step<Op> they carry the Op directly
    type Ffi: Serialize;

    fn serialize<'out>(self) -> (Self::Ffi, Resolve<&'out [u8]>);
}
