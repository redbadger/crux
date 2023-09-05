package com.example.{{workspace}}

import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.mutableStateOf
import com.example.{{workspace}}.{{core_name}}.processEvent
import com.example.{{workspace}}.{{core_name}}.view
import com.example.{{workspace}}.{{type_gen}}.Effect
import com.example.{{workspace}}.{{type_gen}}.Event
import com.example.{{workspace}}.{{type_gen}}.Request
import com.example.{{workspace}}.{{type_gen}}.Requests
import com.example.{{workspace}}.{{type_gen}}.ViewModel

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
