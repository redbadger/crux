package com.crux.examples.simple_counter

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

open class Core : androidx.lifecycle.ViewModel() {
    private var core: CoreFfi = CoreFfi()

    var view: ViewModel by mutableStateOf(
        ViewModel.bincodeDeserialize(core.view())
    )
        private set

    fun update(event: Event) {
        val effects = core.update(event.bincodeSerialize())

        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processEffect(request)
        }
    }

    private fun processEffect(request: Request) {
        when (val effect = request.effect) {
            is Effect.Render -> {
                this.view = ViewModel.bincodeDeserialize(core.view())
            }
        }
    }
}
