package com.example.simple_counter

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.crux.example.simple_counter.Effect
import com.crux.example.simple_counter.Event
import com.crux.example.simple_counter.Request
import com.crux.example.simple_counter.Requests
import com.crux.example.simple_counter.ViewModel
import com.example.simple_counter.shared.processEvent
import com.example.simple_counter.shared.view

class Core : androidx.lifecycle.ViewModel() {
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
