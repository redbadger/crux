package com.crux.example.bridge_echo

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

open class Core : androidx.lifecycle.ViewModel()  {
    private var core: CoreFfi = CoreFfi()
    var view: ViewModel? by mutableStateOf(null)
        private set

    fun update(event: Event) {
        val effects = core.update(event.bincodeSerialize())

        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processEffect(request)
        }
    }

    private fun processEffect(request: Request) {
        when (request.effect) {
            is Effect.Render -> {
                this.view = ViewModel.bincodeDeserialize(core.view())
            }
        }
    }
}
