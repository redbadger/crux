use crate::{
    capability::Operation,
    core::{RequestHandle, ResolveError},
    Request,
};

use super::{BridgeError, FfiFormat};

// used in docs/internals/bridge.md
// ANCHOR: resolve_serialized
type ResolveOnceSerialized<T> = Box<dyn FnOnce(&[u8]) -> Result<(), BridgeError<T>> + Send>;
type ResolveManySerialized<T> = Box<dyn FnMut(&[u8]) -> Result<(), BridgeError<T>> + Send>;

/// A deserializing version of Resolve
///
/// `ResolveSerialized` is a separate type because lifetime elision doesn't work
/// through generic type arguments. We can't create a `ResolveRegistry` of
/// Resolve<&[u8]> without specifying an explicit lifetime.
/// If you see a better way around this, please open a PR.
pub enum ResolveSerialized<T: FfiFormat> {
    Never,
    Once(ResolveOnceSerialized<T>),
    Many(ResolveManySerialized<T>),
}
// ANCHOR_END: resolve_serialized

impl<T: FfiFormat> ResolveSerialized<T> {
    pub(crate) fn resolve(&mut self, response: &[u8]) -> Result<(), BridgeError<T>> {
        match self {
            ResolveSerialized::Never => Err(BridgeError::ProcessResponse(ResolveError::Never)),
            ResolveSerialized::Many(f) => f(response),
            ResolveSerialized::Once(_) => {
                // The resolve has been used, turn it into a Never
                let ResolveSerialized::Once(f) = std::mem::replace(self, ResolveSerialized::Never)
                else {
                    unreachable!("already resolved");
                };

                f(response)
            }
        }
    }
}

impl<Op> Request<Op>
where
    Op: Operation,
    Op::Output: for<'de> serde::de::Deserialize<'de>,
{
    /// Serialize this effect request using `effect` as a constructor
    /// for a serializable Effect `Eff`
    ///
    /// You should never need to call this method yourself, it will be called
    /// by the generated implementation of [`Effect::serialize`](crate::Effect::serialize),
    /// which is used by the Bridge implementation.
    pub fn serialize<F, Eff, T>(self, effect: F) -> (Eff, ResolveSerialized<T>)
    where
        F: FnOnce(Op) -> Eff,
        T: FfiFormat,
    {
        // FIXME should Eff be bound as `Serializable`?
        let handle = self.handle.deserializing(move |response| {
            T::deserialize(response).map_err(BridgeError::DeserializeOutput)
        });

        (effect(self.operation), handle)
    }
}

impl<Out> RequestHandle<Out> {
    /// Convert this Resolve into a version which deserializes from bytes, consuming it.
    /// The `func` argument is a 'deserializer' converting from bytes into the `Out` type.
    fn deserializing<F, T>(self, mut func: F) -> ResolveSerialized<T>
    where
        F: (FnMut(&[u8]) -> Result<Out, BridgeError<T>>) + Send + Sync + 'static,
        T: FfiFormat,
        Out: 'static,
    {
        match self {
            RequestHandle::Never => ResolveSerialized::Never,
            RequestHandle::Once(resolve) => ResolveSerialized::Once(Box::new(move |response| {
                let out = func(response)?;
                resolve(out);
                Ok(())
            })),
            RequestHandle::Many(resolve) => ResolveSerialized::Many(Box::new(move |response| {
                let out = func(response)?;
                resolve(out).map_err(|()| BridgeError::ProcessResponse(ResolveError::FinishedMany))
            })),
        }
    }
}
