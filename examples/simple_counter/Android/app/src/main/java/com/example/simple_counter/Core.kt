package com.example.simple_counter

import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.mutableStateOf
import com.example.simple_counter.shared.processEvent
import com.example.simple_counter.shared.view
import com.example.simple_counter.shared_types.Effect
import com.example.simple_counter.shared_types.Event
import com.example.simple_counter.shared_types.Request
import com.example.simple_counter.shared_types.Requests
import com.example.simple_counter.shared_types.ViewModel

class Core : androidx.lifecycle.ViewModel() {
    var view: ViewModel by mutableStateOf(ViewModel.bincodeDeserialize(view()))
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