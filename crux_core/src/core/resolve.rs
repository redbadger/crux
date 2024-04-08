use thiserror::Error;

// used in docs/internals/runtime.md
// ANCHOR: resolve
type ResolveOnce<Out> = Box<dyn FnOnce(Out) + Send>;
type ResolveMany<Out> = Box<dyn Fn(Out) -> Result<(), ()> + Send>;

/// Resolve is a callback used to resolve an effect request and continue
/// one of the capability Tasks running on the executor.
pub(crate) enum Resolve<Out> {
    Never,
    Once(ResolveOnce<Out>),
    Many(ResolveMany<Out>),
}
// ANCHOR_END: resolve

impl<Out> Resolve<Out> {
    pub fn resolve(&mut self, output: Out) -> Result<(), ResolveError> {
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
}

#[derive(Error, Debug)]
pub enum ResolveError {
    #[error("Attempted to resolve a request that is not expected to be resolved.")]
    Never,
    #[error("Attempted to resolve a request that has concluded.")]
    FinishedMany,
}
