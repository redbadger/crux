use std::sync::Mutex;

use slab::Slab;

use super::request_native::ResolveNative;
use super::{EffectId, NativeBridgeError};

/// Registry for native typed resolve handles, parallel to
/// [`ResolveRegistry`](super::ResolveRegistry).
///
/// Uses a `Slab` behind a `Mutex` for concurrent access from both the
/// bridge's synchronous path and middleware's async callback path.
pub struct NativeResolveRegistry<EffOut: Send + 'static>(Mutex<Slab<ResolveNative<EffOut>>>);

impl<EffOut: Send + 'static> Default for NativeResolveRegistry<EffOut> {
    fn default() -> Self {
        Self(Mutex::new(Slab::with_capacity(1024)))
    }
}

impl<EffOut: Send + 'static> NativeResolveRegistry<EffOut> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a resolver, return the `EffectId`.
    pub fn insert(&self, resolve: ResolveNative<EffOut>) -> EffectId {
        let id = self
            .0
            .lock()
            .expect("NativeResolveRegistry Mutex poisoned")
            .insert(resolve);

        EffectId(id.try_into().expect("EffectId overflow"))
    }

    /// Resume a previously stored resolver with typed output.
    ///
    /// # Registry cleanup
    /// - `Never` entries: removed immediately (fire-and-forget effects)
    /// - `Once` entries: transition to `Never` via `mem::replace` on resolve, then removed
    /// - `Many` entries: stay alive for repeated resolution (streaming effects)
    pub fn resume(&self, id: EffectId, output: EffOut) -> Result<(), NativeBridgeError> {
        let mut lock = self.0.lock().expect("NativeResolveRegistry Mutex poisoned");

        let entry =
            lock.get_mut(id.0 as usize)
                .ok_or_else(|| NativeBridgeError::ProcessResponse {
                    message: format!("effect id {} not found", id.0),
                })?;

        let result = entry.resolve(output);

        // Clean up entries that will never be resolved again
        if let ResolveNative::Never = entry {
            lock.remove(id.0 as usize);
        }

        result
    }
}
