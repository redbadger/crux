package com.crux.examples.counter.middleware

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope

/// Holds the generated `Core`, so the count — and the Server-Sent Events
/// connection — survives a configuration change.
///
/// `viewModelScope` runs on `Dispatchers.Main.immediate` with a
/// `SupervisorJob`, which is what `Core` needs, and is cancelled when
/// Android clears this `CounterViewModel`, which closes the connection.
class CounterViewModel : ViewModel() {
    private val effects = MiddlewareEffects(viewModelScope)
    private val bridge = MiddlewareBridge(effects)

    /// Built over the hand-written bridge rather than the generated
    /// `FfiBridge`, because the middleware needs `CoreFfi` to have a callback.
    val core = Core(bridge, CounterHandler(viewModelScope), viewModelScope)

    init {
        // Close the cycle: from here on the middleware's effects reach `core`.
        effects.core = core
        // Release the Rust core, and with it the callback, when Android
        // clears this `CounterViewModel`.
        addCloseable(bridge)

        // Once per `CounterViewModel`, not per activity, so that a rotation
        // does not open a second stream.
        core.update(Event.STARTWATCH)
    }
}
