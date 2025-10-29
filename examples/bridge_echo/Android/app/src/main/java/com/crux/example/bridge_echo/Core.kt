package com.crux.example.bridge_echo

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.launch

open class Core : androidx.lifecycle.ViewModel(), CruxShell  {
    private var core: CoreFfi = CoreFfi(this)
    var view: ViewModel? by mutableStateOf(null)
        private set

    suspend fun update(event: Event) {
        val effects = core.update(event.bincodeSerialize())

        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processEffect(request)
        }
    }

    private suspend fun processEffect(request: Request) {
        when (request.effect) {
            is Effect.Render -> {
                this.view = ViewModel.bincodeDeserialize(core.view())
            }
        }
    }

    override fun processEffects(bytes: ByteArray) {
        val requests = Requests.bincodeDeserialize(bytes)
        for (request in requests) {
            viewModelScope.launch {
                processEffect(request)
            }
        }
    }
}
