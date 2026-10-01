package com.crux.examples.counter.http

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.crux.examples.counter.Core
import com.crux.examples.counter.Event

/// Holds the generated `Core`, so the count — and the Server-Sent Events
/// connection — survives a configuration change.
///
/// `viewModelScope` runs on `Dispatchers.Main.immediate` with a
/// `SupervisorJob`, which is what `Core` needs, and is cancelled when
/// Android clears this `CounterViewModel`, which closes the connection.
class CounterViewModel : ViewModel() {
    val core = Core(CounterHandler(viewModelScope), viewModelScope)

    init {
        // Once per `CounterViewModel`, not per activity, so that a rotation
        // does not open a second stream.
        core.update(Event.STARTWATCH)
    }
}
