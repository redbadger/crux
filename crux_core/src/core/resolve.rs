use thiserror::Error;

use crate::bridge::EffectId;

// used in docs/internals/runtime.md
// ANCHOR: resolve
type ResolveOnce<Out> = Box<dyn FnOnce(Out) + Send>;
type ResolveMany<Out> = Box<dyn Fn(Out) -> Result<(), ()> + Send>;

/// Resolve is a callback used to resolve an effect request and continue
/// one of the capability Tasks running on the executor.
pub enum RequestHandle<Out> {
    Never,
    Twice,
    Once(ResolveOnce<Out>),
    Many(ResolveMany<Out>),
}
// ANCHOR_END: resolve

pub trait Resolvable<Output> {
    /// Resolve the request with the given output.
    /// # Errors
    /// Returns an error if the request is not expected to be resolved.
    fn resolve(&mut self, output: Output) -> Result<(), ResolveError>;
}

impl<Output> Resolvable<Output> for RequestHandle<Output> {
    fn resolve(&mut self, output: Output) -> Result<(), ResolveError> {
        self.resolve(output)
    }
}

impl<Output> RequestHandle<Output> {
    pub fn resolve(&mut self, output: Output) -> Result<(), ResolveError> {
        match self {
            RequestHandle::Never => Err(ResolveError::Never),
            RequestHandle::Many(f) => f(output).map_err(|()| ResolveError::FinishedMany),
            RequestHandle::Once(_) => {
                // The resolve has been used, turn it into a Never
                if let RequestHandle::Once(f) = std::mem::replace(self, RequestHandle::Twice) {
                    f(output);
                }

                Ok(())
            }
            RequestHandle::Twice =>  Err(ResolveError::Twice),
        }
    }
}

#[derive(Error, Debug)]
pub enum ResolveError {
    #[error("Attempted to resolve a request that is not expected to be resolved.")]
    Never,
    #[error("Attempted to resolve a request that has concluded.")]
    FinishedMany,
    #[error("Attempted to resolve twice a request that is only expected to be resolved once")]
    Twice,
    #[error("Request with {0:?} not found.")]
    NotFound(EffectId), // FIXME: since it uses EffectId, should ResolveError be defined further up the module tree?
}
