@file:OptIn(ExperimentalUnsignedTypes::class)

package com.example.bridge_echo

import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import com.example.bridge_echo.shared.processEvent
import com.example.bridge_echo.shared.view
import com.example.bridge_echo.shared_types.Effect
import com.example.bridge_echo.shared_types.Event
import com.example.bridge_echo.shared_types.Requests
import com.example.bridge_echo.ui.theme.CounterTheme
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import com.example.bridge_echo.shared_types.Event as Evt
import com.example.bridge_echo.shared_types.Request as Req
import com.example.bridge_echo.shared_types.ViewModel as MyViewModel


class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            CounterTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    View()
                }
            }
        }
    }
}

sealed class CoreMessage {
    data class Event(val event: Evt) : CoreMessage()
}

class Model : ViewModel() {
    var view: MyViewModel by mutableStateOf(MyViewModel(0))
        private set

    init {
        viewModelScope.launch {
            clock()
        }
        viewModelScope.launch {
            ticker()
        }
    }

    suspend fun ticker() {
        withContext(Dispatchers.Default) {
            while (true) {
                update(CoreMessage.Event(Event.Tick()))
            }
        }
    }

    suspend fun clock() {
        withContext(Dispatchers.Default) {
            while (true) {
                update(CoreMessage.Event(Event.NewPeriod()))
                delay(1000)
            }
        }
    }

    fun update(msg: CoreMessage) {
        val requests: List<Req> =
            when (msg) {
                is CoreMessage.Event -> {
                    Requests.bincodeDeserialize(
                        processEvent(msg.event.bincodeSerialize())
                    )
                }
            }

        for (req in requests) when (req.effect) {
            is Effect.Render -> {
                this.view = MyViewModel.bincodeDeserialize(view())
            }
        }
    }
}

@Composable
fun View(model: Model = viewModel()) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
        modifier = Modifier
            .fillMaxSize()
            .padding(10.dp),
    ) {
        Text(text = model.view.count.toString(), fontSize = 30.sp,  modifier = Modifier.padding(10.dp))
    }
}

@Preview(showBackground = true)
@Composable
fun DefaultPreview() {
    CounterTheme {
        View()
    }
}
