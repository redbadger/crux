use thiserror::Error;

use crate::{MaybeSend, bridge::EffectId};

pub trait ResolveOnceFn<Out>: FnOnce(Out) + MaybeSend {}
impl<Out, T> ResolveOnceFn<Out> for T where T: FnOnce(Out) + MaybeSend {}

pub trait ResolveManyFn<Out>: Fn(Out) -> Result<(), ()> + MaybeSend {}
impl<Out, T> ResolveManyFn<Out> for T where T: Fn(Out) -> Result<(), ()> + MaybeSend {}

// used in docs/internals/runtime.md
// ANCHOR: resolve
type ResolveOnce<Out> = Box<dyn ResolveOnceFn<Out>>;
type ResolveMany<Out> = Box<dyn ResolveManyFn<Out>>;

/// Resolve is a callback used to resolve an effect request and continue
/// one of the capability Tasks running on the executor.
pub enum RequestHandle<Out> {
    Never,
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
    /// Resolve the request with the given output.
    /// # Errors
    /// Returns an error if the request is not expected to be resolved.
    pub fn resolve(&mut self, output: Output) -> Result<(), ResolveError> {
        match self {
            RequestHandle::Never => Err(ResolveError::Never),
            RequestHandle::Many(f) => f(output).map_err(|()| ResolveError::FinishedMany),
            RequestHandle::Once(_) => {
                // The resolve has been used, turn it into a Never
                if let RequestHandle::Once(f) = std::mem::replace(self, RequestHandle::Never) {
                    f(output);
                }

                Ok(())
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum ResolveError {
    #[error("Attempted to resolve a request that is not expected to be resolved.")]
    Never,
    #[error("Attempted to resolve a request that has concluded.")]
    FinishedMany,
    #[error("Request with {0:?} not found.")]
    NotFound(EffectId), // FIXME: since it uses EffectId, should ResolveError be defined further up the module tree?
}
