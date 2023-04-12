use crate::{
    capability::Operation,
    core::{Resolve, ResolveError},
    Request,
};

type ResolveOnceBytes = Box<dyn FnOnce(&[u8]) + Send>;
type ResolveManyBytes = Box<dyn Fn(&[u8]) -> Result<(), ()> + Send>;

/// A deserializing version of Resolve
///
/// ResolveBytes is a separate type because lifetime elision doesn't work
/// through generic type arguments. We can't create a ResolveRegistry of
/// Resolve<&[u8]> without specifying an explicit lifetime.
/// If you see a better way around this, please open a PR.
pub enum ResolveBytes {
    Never,
    Once(ResolveOnceBytes),
    Many(ResolveManyBytes),
}

impl ResolveBytes {
    pub(crate) fn resolve(&mut self, bytes: &[u8]) -> Result<(), ResolveError> {
        match self {
            ResolveBytes::Never => Err(ResolveError::Never),
            ResolveBytes::Many(f) => f(bytes).map_err(|_| ResolveError::FinishedMany),
            ResolveBytes::Once(_) => {
                // The resolve has been used, turn it into a Never
                if let ResolveBytes::Once(f) = std::mem::replace(self, ResolveBytes::Never) {
                    f(bytes);
                }

                Ok(())
            }
        }
    }
}

impl<Op> Request<Op>
where
    Op: Operation,
{
    /// Serialize this effect request using `effect` as a constructor
    /// for a serializable Effect `Eff`
    pub fn serialize<F, Eff>(self, effect: F) -> (Eff, ResolveBytes)
    where
        F: Fn(Op) -> Eff,
    {
        // FIXME should Eff be bound as `Serializable`?
        let (operation, resolve) = (self.operation, self.resolve);

        let resolve =
            resolve.deserializing(|bytes| bcs::from_bytes(bytes).expect("Deserialization failed"));

        (effect(operation), resolve)
    }
}

impl<Out> Resolve<Out> {
    /// Convert this Resolve into a version which deserializes from bytes, consuming it.
    /// The `func` argument is a 'deserializer' converting from bytes into the `Out` type.
    fn deserializing<F>(self, func: F) -> ResolveBytes
    where
        F: (Fn(&[u8]) -> Out) + Send + Sync + 'static,
        Out: 'static,
    {
        match self {
            Resolve::Never => ResolveBytes::Never,
            Resolve::Once(resolve) => ResolveBytes::Once(Box::new(move |bytes| {
                let out = func(bytes);
                resolve(out)
            })),
            Resolve::Many(resolve) => ResolveBytes::Many(Box::new(move |bytes| {
                let out = func(bytes);
                resolve(out)
            })),
        }
    }
}
