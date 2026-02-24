use crate::capability::Operation;
use crate::core::RequestHandle;

use super::NativeBridgeError;

type ResolveOnceNative<T> = Box<dyn FnOnce(T) -> Result<(), NativeBridgeError> + Send>;
type ResolveManyNative<T> = Box<dyn FnMut(T) -> Result<(), NativeBridgeError> + Send>;

/// A typed version of resolve, parallel to [`ResolveSerialized`](super::ResolveSerialized).
///
/// Instead of deserializing from bytes, `ResolveNative` uses a typed extractor
/// to convert from the `EffectOutput` sum type to the specific `Op::Output`.
pub enum ResolveNative<EffOut: Send + 'static> {
    Never,
    Once(ResolveOnceNative<EffOut>),
    Many(ResolveManyNative<EffOut>),
}

impl<EffOut: Send + 'static> ResolveNative<EffOut> {
    pub(crate) fn resolve(&mut self, output: EffOut) -> Result<(), NativeBridgeError> {
        match self {
            ResolveNative::Never => Err(NativeBridgeError::ResolveNever),
            ResolveNative::Once(_) => {
                // Take ownership by replacing with Never
                let ResolveNative::Once(f) = std::mem::replace(self, ResolveNative::Never) else {
                    unreachable!()
                };
                f(output)
            }
            ResolveNative::Many(f) => f(output),
        }
    }
}

impl<Op> crate::Request<Op>
where
    Op: Operation,
    Op::Output: 'static,
{
    /// Convert this request into its native FFI representation using `effect_fn`
    /// as a constructor for the FFI effect, and `extractor` to convert from
    /// the typed `EffectOutput` enum to the specific `Op::Output`.
    ///
    /// This is the native parallel to [`Request::serialize`](crate::Request::serialize).
    pub fn into_native<F, Eff, EffOut>(
        self,
        effect_fn: F,
        extractor: fn(EffOut) -> Result<Op::Output, NativeBridgeError>,
    ) -> (Eff, ResolveNative<EffOut>)
    where
        F: FnOnce(Op) -> Eff,
        EffOut: Send + 'static,
    {
        let handle = self.handle.extracting(extractor);
        (effect_fn(self.operation), handle)
    }
}

impl<Out: 'static> RequestHandle<Out> {
    /// Convert this RequestHandle into a `ResolveNative` which uses a typed
    /// extractor function instead of byte deserialization.
    ///
    /// This is the native parallel to [`RequestHandle::deserializing`].
    fn extracting<EffOut, F>(self, mut extractor: F) -> ResolveNative<EffOut>
    where
        F: (FnMut(EffOut) -> Result<Out, NativeBridgeError>) + Send + 'static,
        EffOut: Send + 'static,
    {
        match self {
            RequestHandle::Never => ResolveNative::Never,
            RequestHandle::Once(resolve) => ResolveNative::Once(Box::new(move |output| {
                let out = extractor(output)?;
                resolve(out);
                Ok(())
            })),
            RequestHandle::Many(resolve) => ResolveNative::Many(Box::new(move |output| {
                let out = extractor(output)?;
                resolve(out).map_err(|()| NativeBridgeError::ResolveFinished)
            })),
        }
    }
}
