package com.crux.examples.counter.routing

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope

/// An Android `ViewModel` (not the app's `ViewModel`, which `Core.view`
/// publishes) that holds the generated `Core`, so the count and the
/// Server-Sent Events connection survive a configuration change.
///
/// `viewModelScope` runs on `Dispatchers.Main.immediate` with a
/// `SupervisorJob`, which is what `Core` needs, and is cancelled when
/// Android clears this `CounterViewModel`, which closes the connection.
class CounterViewModel : ViewModel() {
    private val effects = RoutedEffects(viewModelScope)
    private val bridge = RoutingBridge(effects)

    /// Built over the hand-written bridge rather than the generated
    /// `FfiBridge`, because the router needs `CoreFfi` to have a callback.
    val core = Core(bridge, CounterHandler(viewModelScope), viewModelScope)

    init {
        // Close the cycle: from here on the routed effects reach `core`.
        effects.core = core
        // Release the Rust core, and with it the callback, when Android
        // clears this `CounterViewModel`.
        addCloseable(bridge)

        // Once per `CounterViewModel`, not per activity, so that a rotation
        // does not open a second stream.
        core.update(Event.STARTWATCH)
    }
}
