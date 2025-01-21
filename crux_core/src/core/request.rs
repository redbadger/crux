use std::fmt::{self, Debug};

use crate::{
    capability::Operation,
    core::resolve::{Resolve, ResolveError},
};

/// Request represents an effect request from the core to the shell.
///
/// The `operation` is the input needed to process the effect, and will be one
/// of the capabilities' [`Operation`] types.
///
/// The request can be resolved by passing it to `Core::resolve` along with the
/// corresponding result of type `Operation::Output`.
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

    /// Resolve the request with the given output.
    ///
    /// Note: This method should only be used in tests that work directly with
    /// [`Command`](crate::command::Command).
    /// If you are using [`AppTester`](crate::testing::AppTester) to test your app,
    /// you should use [`AppTester::resolve`](crate::testing::AppTester::resolve) instead.
    pub fn resolve(&mut self, output: Op::Output) -> Result<(), ResolveError> {
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
