@file:OptIn(ExperimentalUnsignedTypes::class)

package com.example.counter

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import com.example.counter.shared.handleResponse
import com.example.counter.shared.processEvent
import com.example.counter.shared.view
import com.example.counter.shared_types.Effect
import com.example.counter.shared_types.HttpResponse
import com.example.counter.shared_types.Requests
import com.example.counter.shared_types.SseResponse
import com.example.counter.ui.theme.CounterTheme
import io.ktor.client.*
import io.ktor.client.engine.cio.*
import io.ktor.http.*
import kotlinx.coroutines.launch
import com.example.counter.shared_types.Event as Evt
import com.example.counter.shared_types.Request as Req
import com.example.counter.shared_types.ViewModel as MyViewModel

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            CounterTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) { View() }
            }
        }
    }
}

sealed class Outcome {
    data class Http(val res: HttpResponse) : Outcome()
    data class Sse(val res: SseResponse) : Outcome()
}

sealed class CoreMessage {
    data class Event(val event: Evt) : CoreMessage()
    data class Response(val uuid: List<UByte>, val outcome: Outcome) : CoreMessage()
}

class Model : ViewModel() {
    var view: MyViewModel by mutableStateOf(MyViewModel("", false))
        private set

    private val httpClient = HttpClient(CIO)
    private val sseClient = HttpClient(CIO) {
        engine {
            endpoint {
                keepAliveTime = 5000
                connectTimeout = 5000
                connectAttempts = 5
                requestTimeout = 0
            }
        }
    }

    init {
        viewModelScope.launch {
            update(CoreMessage.Event(Evt.StartWatch()))
        }
    }

    suspend fun update(msg: CoreMessage) {
        val requests: List<Req> =
            when (msg) {
                is CoreMessage.Event ->
                    Requests.bincodeDeserialize(
                        processEvent(msg.event.bincodeSerialize().toUByteArray().toList())
                            .toUByteArray()
                            .toByteArray()
                    )
                is CoreMessage.Response ->
                    Requests.bincodeDeserialize(
                        handleResponse(
                            msg.uuid.toList(),
                            when (msg.outcome) {
                                is Outcome.Http -> msg.outcome.res.bincodeSerialize()
                                is Outcome.Sse -> msg.outcome.res.bincodeSerialize()
                            }.toUByteArray().toList()
                        ).toUByteArray().toByteArray()
                    )
            }

        for (req in requests) when (val effect = req.effect) {
            is Effect.Render -> {
                this.view = MyViewModel.bincodeDeserialize(view().toUByteArray().toByteArray())
            }
            is Effect.Http -> {
                val response = http(httpClient, HttpMethod(effect.value.method), effect.value.url, effect.value.headers)
                update(
                    CoreMessage.Response(
                        req.uuid.toByteArray().toUByteArray().toList(),
                        Outcome.Http(response)
                    )
                )
            }
            is Effect.ServerSentEvents -> {
                viewModelScope.launch {
                    sse(sseClient, effect.value.url) { event ->
                        update(
                            CoreMessage.Response(
                                req.uuid.toByteArray().toUByteArray().toList(),
                                Outcome.Sse(event)
                            )
                        )
                    }
                }
            }
        }
    }
}


@Composable
fun View(model: Model = viewModel()) {
    val coroutineScope = rememberCoroutineScope()
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
        modifier = Modifier
            .fillMaxSize()
            .padding(10.dp),
    ) {
        Text(text = "Crux Counter Example", fontSize = 30.sp, modifier = Modifier.padding(10.dp))
        Text(text = "Rust Core, Kotlin Shell (Jetpack Compose)", modifier = Modifier.padding(10.dp))
        Text(text = model.view.text, color = if(model.view.confirmed) { Color.Black } else { Color.Gray }, modifier = Modifier.padding(10.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Button(
                onClick = { coroutineScope.launch { model.update(CoreMessage.Event(Evt.Decrement())) } },
                colors = ButtonDefaults.buttonColors(containerColor = Color.hsl(44F, 1F, 0.77F))
            ) { Text(text = "Decrement", color = Color.DarkGray) }
            Button(
                onClick = { coroutineScope.launch { model.update(CoreMessage.Event(Evt.Increment())) } },
                colors =
                ButtonDefaults.buttonColors(
                    containerColor = Color.hsl(348F, 0.86F, 0.61F)
                )
            ) { Text(text = "Increment", color = Color.White) }
        }
    }
}

@Preview(showBackground = true)
@Composable
fun DefaultPreview() {
    CounterTheme { View() }
}
