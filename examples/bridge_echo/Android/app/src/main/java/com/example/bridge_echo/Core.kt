package com.example.bridge_echo

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.example.bridge_echo.shared.processEvent
import com.example.bridge_echo.shared.view
import com.crux.example.bridge_echo.Effect
import com.crux.example.bridge_echo.Event
import com.crux.example.bridge_echo.Request
import com.crux.example.bridge_echo.Requests
import com.crux.example.bridge_echo.ViewModel


open class Core : androidx.lifecycle.ViewModel() {
    var view: ViewModel? by mutableStateOf(null)
        private set

    fun update(event: Event) {
        val effects = processEvent(event.bincodeSerialize())

        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processEffect(request)
        }
    }

    private fun processEffect(request: Request) {
        when (request.effect) {
            is Effect.Render -> {
                this.view = ViewModel.bincodeDeserialize(view())
            }
        }
    }
}
