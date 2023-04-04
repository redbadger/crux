use std::fmt::{self, Debug};

use crate::{
    capability::Operation,
    core::resolve::{Resolve, ResolveError},
};

pub struct Request<Op>
where
    Op: Operation,
{
    pub operation: Op,
    pub(crate) resolve: Resolve<Op::Output>,
}

impl<Op> Request<Op>
where
    Op: Operation,
{
    pub(crate) fn resolves_never(operation: Op) -> Self {
        Self {
            operation,
            resolve: Resolve::Never,
        }
    }

    pub(crate) fn resolves_once<F>(operation: Op, resolve: F) -> Self
    where
        F: FnOnce(Op::Output) + Send + 'static,
    {
        Self {
            operation,
            resolve: Resolve::Once(Box::new(resolve)),
        }
    }

    pub(crate) fn resolves_many_times<F>(operation: Op, resolve: F) -> Self
    where
        F: Fn(Op::Output) -> Result<(), ()> + Send + 'static,
    {
        Self {
            operation,
            resolve: Resolve::Many(Box::new(resolve)),
        }
    }

    pub(crate) fn resolve(&mut self, output: Op::Output) -> Result<(), ResolveError> {
        self.resolve.resolve(output)
    }
}

impl<Op> fmt::Debug for Request<Op>
where
    Op: Operation + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Request").field(&self.operation).finish()
    }
}
