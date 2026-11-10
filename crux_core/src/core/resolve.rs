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
/// Where an [`Operation`](crate::capability::Operation) declares
/// [`KIND`](crate::capability::Operation::KIND), this is a static property of
/// the operation type, and every request carrying that operation has the same
/// kind. An operation that has not declared one leaves the kind to the call
/// that created the request — `notify_shell`, `request_from_shell` or
/// `stream_from_shell` — so one such operation type can be notified in one
/// place and streamed in another.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Facet, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(C)]
pub enum OperationKind {
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
    /// reports [`OperationKind::Notify`] once answered. Read it before resolving.
    #[must_use]
    pub const fn kind(&self) -> OperationKind {
        match self {
            Self::Never => OperationKind::Notify,
            Self::Once(_) => OperationKind::Request,
            Self::Many(_) => OperationKind::Stream,
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

/// One variant of an effect enum, as an error can name it.
///
/// A request id carries only the variant's index; the name comes from
/// [`EffectFFI::variant_name`](crate::EffectFFI::variant_name), which the
/// `#[effect]` macro implements and a hand-written effect may not, hence the
/// [`Option`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectVariant {
    /// Which variant this is, counting from zero in declaration order.
    pub index: u8,
    /// What the variant is called, where the effect implementation knows.
    pub name: Option<&'static str>,
}

impl std::fmt::Display for EffectVariant {
    /// Renders as ``` `KvGet` (variant 1) ``` where the name is known, and
    /// `variant 1` where it is not.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = self.name {
            write!(f, "`{name}` (variant {})", self.index)
        } else {
            write!(f, "variant {}", self.index)
        }
    }
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ResolveError {
    #[error("Attempted to resolve a request that is not expected to be resolved.")]
    Never,
    #[error("Attempted to resolve a request that has concluded.")]
    FinishedMany,
    #[error("Request with id {0} not found.")]
    NotFound(u64),
    /// The id carries an effect variant index the effect enum does not have,
    /// so it cannot be one the bridge issued.
    #[error(
        "`{effect}` has only {variants} variants, but response id {id:#010x} carries variant {index}."
    )]
    NoSuchEffect {
        id: u32,
        index: u8,
        variants: u16,
        effect: &'static str,
    },
    /// The id's sequence is outstanding, but it was issued for a different
    /// effect variant.
    #[error("Request {sequence} expects {expected}, but response id {id:#010x} carries {actual}.")]
    WrongEffect {
        id: u32,
        sequence: u32,
        expected: EffectVariant,
        actual: EffectVariant,
    },
    /// The id's sequence is outstanding, but it was issued as a different
    /// [`OperationKind`].
    #[error(
        "Request {sequence} expects the {expected:?} kind, but response id {id:#010x} carries the {actual:?} kind."
    )]
    WrongKind {
        id: u32,
        sequence: u32,
        expected: OperationKind,
        actual: OperationKind,
    },
}
