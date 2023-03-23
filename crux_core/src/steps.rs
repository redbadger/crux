use std::fmt::{self, Debug};

use crate::capability::Operation;

type ResolveOnce<Out> = Box<dyn FnOnce(Out) + Send>;
type ResolveMany<Out> = Box<dyn Fn(Out) -> Result<(), ()> + Send>;

#[derive(Debug)]
pub enum Step<Op>
where
    Op: Operation,
{
    Never(StepNever<Op>),
    Once(StepOnce<Op>),
    Many(StepMany<Op>),
}

impl<Op> Step<Op>
where
    Op: Operation,
{
    pub(crate) fn resolves_never(payload: Op) -> Self {
        Self::Never(StepNever { payload })
    }

    pub(crate) fn resolves_once<F>(payload: Op, resolve: F) -> Self
    where
        F: FnOnce(Op::Output) + Send + 'static,
    {
        Self::Once(StepOnce {
            payload,
            callback: Box::new(resolve),
        })
    }

    pub(crate) fn resolves_many_times<F>(payload: Op, advance: F) -> Self
    where
        F: Fn(Op::Output) -> Result<(), ()> + Send + 'static,
    {
        Self::Many(StepMany {
            payload,
            callback: Box::new(advance),
        })
    }
}

#[derive(Debug)]
pub struct StepNever<Op>
where
    Op: Operation,
{
    pub payload: Op,
}

pub struct StepOnce<Op>
where
    Op: Operation,
{
    pub payload: Op,
    callback: ResolveOnce<Op::Output>,
}

impl<Op> StepOnce<Op>
where
    Op: Operation,
{
    pub fn resolve(self, output: Op::Output) {
        (self.callback)(output);
    }
}

pub struct StepMany<Op>
where
    Op: Operation,
{
    payload: Op,
    callback: ResolveMany<Op::Output>,
}

impl<Op> StepMany<Op>
where
    Op: Operation,
{
    // TODO should this be called something else to be more explicit?
    pub fn resolve(&self, output: Op::Output) -> Result<(), ()> {
        (self.callback)(output)
    }
}

impl<Op> fmt::Debug for StepOnce<Op>
where
    Op: Operation + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StepOnce")
            .field("payload", &self.payload)
            .finish_non_exhaustive()
    }
}

impl<Op> fmt::Debug for StepMany<Op>
where
    Op: Operation + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StepOnce")
            .field("payload", &self.payload)
            .finish_non_exhaustive()
    }
}
