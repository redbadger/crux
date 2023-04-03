use std::fmt::{self, Debug};
use thiserror::Error;

use crate::capability::Operation;

pub struct Step<Op>(pub Op, pub Resolve<Op::Output>)
where
    Op: Operation;

impl<Op> Step<Op>
where
    Op: Operation,
{
    pub(crate) fn resolves_never(payload: Op) -> Self {
        Self(payload, Resolve::Never)
    }

    pub(crate) fn resolves_once<F>(payload: Op, resolve: F) -> Self
    where
        F: FnOnce(Op::Output) + Send + 'static,
    {
        Self(payload, Resolve::Once(Box::new(resolve)))
    }

    pub(crate) fn resolves_many_times<F>(payload: Op, resolve: F) -> Self
    where
        F: Fn(Op::Output) -> Result<(), ()> + Send + 'static,
    {
        Self(payload, Resolve::Many(Box::new(resolve)))
    }

    pub(crate) fn resolve(&mut self, output: Op::Output) -> Result<(), ResolveError> {
        self.1.resolve(output)
    }
}

impl<Op> fmt::Debug for Step<Op>
where
    Op: Operation + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Step").field(&self.0).finish()
    }
}

impl<Op> PartialEq for Step<Op>
where
    Op: Operation,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<Op> Eq for Step<Op> where Op: Operation {}

type ResolveOnce<Out> = Box<dyn FnOnce(Out) + Send>;
type ResolveMany<Out> = Box<dyn Fn(Out) -> Result<(), ()> + Send>;

pub enum Resolve<Out> {
    Never,
    Once(ResolveOnce<Out>),
    Many(ResolveMany<Out>),
}

impl<Out> Resolve<Out> {
    pub(crate) fn resolve(&mut self, output: Out) -> Result<(), ResolveError> {
        match self {
            Resolve::Never => Err(ResolveError::Never),
            Resolve::Many(f) => f(output).map_err(|_| ResolveError::FinishedMany),
            Resolve::Once(_) => {
                // The resolve has been used, turn it into a Never
                if let Resolve::Once(f) = std::mem::replace(self, Resolve::Never) {
                    f(output);
                }

                Ok(())
            }
        }
    }

    pub(crate) fn map<F, Other>(self, func: F) -> Resolve<Other>
    where
        F: (Fn(Other) -> Out) + Send + Sync + 'static,
        Out: 'static,
    {
        match self {
            Resolve::Never => Resolve::Never,
            Resolve::Once(resolve) => Resolve::Once(Box::new(move |other: Other| {
                let out = func(other);
                resolve(out)
            })),
            Resolve::Many(resolve) => Resolve::Many(Box::new(move |other: Other| {
                let out = func(other);
                resolve(out)
            })),
        }
    }
}

#[derive(Error, Debug)]
pub enum ResolveError {
    #[error("Attempted to resolve a step that is not expected to be resolved.")]
    Never,
    #[error("Attempted to resolve a step that has concluded.")]
    FinishedMany,
}
