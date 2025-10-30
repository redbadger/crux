use crate::{
    MaybeSend, MaybeSync, Request,
    capability::Operation,
    core::{RequestHandle, ResolveError},
};

use super::BridgeError;

pub trait ResolveOnceFn:
    FnOnce(&mut dyn erased_serde::Deserializer) -> Result<(), BridgeError> + MaybeSend
{
}

impl<T> ResolveOnceFn for T where
    T: FnOnce(&mut dyn erased_serde::Deserializer) -> Result<(), BridgeError> + MaybeSend
{
}

pub trait ResolveManyFn:
    FnMut(&mut dyn erased_serde::Deserializer) -> Result<(), BridgeError> + MaybeSend
{
}

impl<T> ResolveManyFn for T where
    T: FnMut(&mut dyn erased_serde::Deserializer) -> Result<(), BridgeError> + MaybeSend
{
}

// used in docs/internals/bridge.md
// ANCHOR: resolve_serialized
type ResolveOnceSerialized = Box<dyn ResolveOnceFn>;
type ResolveManySerialized = Box<dyn ResolveManyFn>;

/// A deserializing version of Resolve
///
/// `ResolveSerialized` is a separate type because lifetime elision doesn't work
/// through generic type arguments. We can't create a `ResolveRegistry` of
/// Resolve<&[u8]> without specifying an explicit lifetime.
/// If you see a better way around this, please open a PR.
pub enum ResolveSerialized {
    Never,
    Once(ResolveOnceSerialized),
    Many(ResolveManySerialized),
}
// ANCHOR_END: resolve_serialized

impl ResolveSerialized {
    pub(crate) fn resolve(
        &mut self,
        bytes: &mut dyn erased_serde::Deserializer,
    ) -> Result<(), BridgeError> {
        match self {
            ResolveSerialized::Never => Err(BridgeError::ProcessResponse(ResolveError::Never)),
            ResolveSerialized::Many(f) => f(bytes),
            ResolveSerialized::Once(_) => {
                // The resolve has been used, turn it into a Never
                let ResolveSerialized::Once(f) = std::mem::replace(self, ResolveSerialized::Never)
                else {
                    unreachable!();
                };

                f(bytes)
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
    pub fn serialize<F, Eff>(self, effect: F) -> (Eff, ResolveSerialized)
    where
        F: FnOnce(Op) -> Eff,
    {
        // FIXME should Eff be bound as `Serializable`?
        let handle = self.handle.deserializing(move |deserializer| {
            erased_serde::deserialize(deserializer).map_err(BridgeError::DeserializeOutput)
        });

        (effect(self.operation), handle)
    }
}

impl<Out> RequestHandle<Out> {
    /// Convert this Resolve into a version which deserializes from bytes, consuming it.
    /// The `func` argument is a 'deserializer' converting from bytes into the `Out` type.
    fn deserializing<F>(self, mut func: F) -> ResolveSerialized
    where
        F: (FnMut(&mut dyn erased_serde::Deserializer) -> Result<Out, BridgeError>)
            + MaybeSend
            + MaybeSync
            + 'static,
        Out: 'static,
    {
        match self {
            RequestHandle::Never => ResolveSerialized::Never,
            RequestHandle::Once(resolve) => ResolveSerialized::Once(Box::new(move |deser| {
                let out = func(deser)?;
                resolve(out);
                Ok(())
            })),
            RequestHandle::Many(resolve) => ResolveSerialized::Many(Box::new(move |deser| {
                let out = func(deser)?;
                resolve(out).map_err(|()| BridgeError::ProcessResponse(ResolveError::FinishedMany))
            })),
        }
    }
}
