use std::{
    fmt::{self, Debug},
    marker::PhantomData,
};

use crate::{
    capability::Operation,
    core::resolve::{Resolve, ResolveError},
};

/// A type that act like an operation `T`.
///
/// See [`Request::take_operation`] for details.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PhantomOperation<T: Operation>(PhantomData<T>);

impl<T: Operation> Operation for PhantomOperation<T> {
    type Output = T::Output;
}

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
    ///
    /// # Errors
    ///
    /// Errors if the request cannot (or should not) be resolved.
    pub fn resolve(&mut self, output: Op::Output) -> Result<(), ResolveError> {
        self.resolve.resolve(output)
    }

    /// Take the operation from the request leaving an [`PhantomOperation<Op>`]
    /// value in its place.
    pub fn take_operation(self) -> (Op, Request<PhantomOperation<Op>>) {
        (
            self.operation,
            Request {
                operation: PhantomOperation(PhantomData),
                resolve: self.resolve,
            },
        )
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

#[cfg(test)]
mod tests {
    use super::Request;
    use crate::capability::Operation;

    #[test]
    fn test_take_operation() {
        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
        pub struct TestOperation;

        #[derive(Debug, serde::Deserialize)]
        pub struct TestOutput;

        impl Operation for TestOperation {
            type Output = TestOutput;
        }

        let request = Request::resolves_never(TestOperation);

        let (operation, mut request) = request.take_operation();
        let _ = request.resolve(TestOutput);

        assert_eq!(operation, TestOperation);
    }
}
