use std::fmt::{self, Debug};
use thiserror::Error;

use serde::Serialize;

use crate::capability::Operation;

type ResolveOnce<Out> = Box<dyn FnOnce(Out) + Send>;
type ResolveMany<Out> = Box<dyn Fn(Out) -> Result<(), ()> + Send>;

#[derive(Serialize)]
pub struct Step<Op>(pub Op, #[serde(skip)] pub Option<Resolve<Op::Output>>)
where
    Op: Operation;

pub enum Resolve<Out> {
    Once(ResolveOnce<Out>),
    Many(ResolveMany<Out>),
}

impl<Op> Step<Op>
where
    Op: Operation,
{
    pub(crate) fn resolves_never(payload: Op) -> Self {
        Self(payload, None)
    }

    pub(crate) fn resolves_once<F>(payload: Op, resolve: F) -> Self
    where
        F: FnOnce(Op::Output) + Send + 'static,
    {
        Self(payload, Some(Resolve::Once(Box::new(resolve))))
    }

    pub(crate) fn resolves_many_times<F>(payload: Op, resolve: F) -> Self
    where
        F: Fn(Op::Output) -> Result<(), ()> + Send + 'static,
    {
        Self(payload, Some(Resolve::Many(Box::new(resolve))))
    }

    pub fn resolve(&mut self, output: Op::Output) -> Result<(), ResolveError> {
        match &self.1 {
            None => Err(ResolveError::Never),
            Some(Resolve::Once(_)) => {
                let resolve = self.1.take(); // Turn the step into a "never"

                if let Some(Resolve::Once(f)) = resolve {
                    f(output);

                    Ok(())
                } else {
                    unreachable!();
                }
            }
            Some(Resolve::Many(f)) => f(output).map_err(|_| ResolveError::FinishedMany),
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
