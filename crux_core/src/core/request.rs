use std::fmt::{self, Debug};

use crate::{
    capability::Operation,
    core::resolve::{Resolve, ResolveError},
};

pub struct Request<Op>(pub Op, pub Resolve<Op::Output>)
where
    Op: Operation;

impl<Op> Request<Op>
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

impl<Op> fmt::Debug for Request<Op>
where
    Op: Operation + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Request").field(&self.0).finish()
    }
}
