package com.crux.examples.counter

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope

/// An Android `ViewModel` (not the app's `ViewModel`, which `Core.view`
/// publishes) that holds the generated `Core`, so the count survives a
/// configuration change.
///
/// `viewModelScope` runs on `Dispatchers.Main.immediate` with a
/// `SupervisorJob`, which is what `Core` needs, and is cancelled when
/// Android clears this `CounterViewModel`.
class CounterViewModel : ViewModel() {
    val core = Core(CounterHandler, viewModelScope)
}
