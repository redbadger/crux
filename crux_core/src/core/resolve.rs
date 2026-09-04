use facet::Facet;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// used in docs/internals/runtime.md
// ANCHOR: resolve
type ResolveOnce<Out> = Box<dyn FnOnce(Out) + Send>;
type ResolveMany<Out> = Box<dyn Fn(Out) -> Result<(), ()> + Send>;

/// Resolve is a callback used to resolve an effect request and continue
/// one of the capability Tasks running on the executor.
pub enum RequestHandle<Out> {
    Never,
    Once(ResolveOnce<Out>),
    Many(ResolveMany<Out>),
}
// ANCHOR_END: resolve

/// How many times a request expects to be resolved.
///
/// Decided by the call that created the request, not by the
/// [`Operation`](crate::capability::Operation) it carries — one operation type
/// can be notified in one place and streamed in another.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Facet, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(C)]
pub enum RequestKind {
    /// The request will never be resolved, and nothing waits on it.
    Notify,
    /// The request expects exactly one response.
    Request,
    /// The request expects a sequence of responses.
    Stream,
}

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
    /// How many times this request expects to be resolved.
    ///
    /// A [`Self::Once`] becomes a [`Self::Never`] as it resolves, so this
    /// reports [`RequestKind::Notify`] once answered. Read it before resolving.
    #[must_use]
    pub const fn kind(&self) -> RequestKind {
        match self {
            Self::Never => RequestKind::Notify,
            Self::Once(_) => RequestKind::Request,
            Self::Many(_) => RequestKind::Stream,
        }
    }

    /// Resolve the request with the given output.
    /// # Errors
    /// Returns an error if the request is not expected to be resolved.
    pub fn resolve(&mut self, output: Output) -> Result<(), ResolveError> {
        match self {
            Self::Never => Err(ResolveError::Never),
            Self::Many(f) => f(output).map_err(|()| ResolveError::FinishedMany),
            Self::Once(_) => {
                // The resolve has been used, turn it into a Never
                if let Self::Once(f) = std::mem::replace(self, Self::Never) {
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
    #[error("Request with id {0} not found.")]
    NotFound(u64),
}
