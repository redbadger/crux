@file:OptIn(ExperimentalUnsignedTypes::class)

package com.example.counter

import android.os.Bundle
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import com.example.counter.shared.handleResponse
import com.example.counter.shared.processEvent
import com.example.counter.shared.view
import com.example.counter.shared_types.Effect
import com.example.counter.shared_types.HttpResponse
import com.example.counter.shared_types.Requests
import com.example.counter.shared_types.SseResponse
import com.example.counter.shared_types.SseResponse.Chunk
import com.example.counter.ui.theme.CounterTheme
import okhttp3.*
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.BufferedReader
import java.io.IOException
import java.io.InputStreamReader
import java.util.concurrent.TimeUnit
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
    var view: MyViewModel by mutableStateOf(MyViewModel(""))
        private set
    private val client = OkHttpClient()

    init {
        update(CoreMessage.Event(Evt.StartWatch()))
    }

    fun update(msg: CoreMessage) {
        val requests: List<Req> =
            when (msg) {
                is CoreMessage.Event ->
                    Requests.bcsDeserialize(
                        processEvent(msg.event.bcsSerialize().toUByteArray().toList())
                            .toUByteArray()
                            .toByteArray()
                    )
                is CoreMessage.Response ->
                    Requests.bcsDeserialize(
                        handleResponse(
                            msg.uuid.toList(),
                            when (msg.outcome) {
                                is Outcome.Http -> msg.outcome.res.bcsSerialize()
                                is Outcome.Sse -> msg.outcome.res.bcsSerialize()
                            }.toUByteArray().toList()
                        ).toUByteArray().toByteArray()
                    )
            }

        for (req in requests) when (val effect = req.effect) {
            is Effect.Render -> {
                this.view = MyViewModel.bcsDeserialize(view().toUByteArray().toByteArray())
            }
            is Effect.Http -> http(req.uuid, effect.value.method, effect.value.url)
            is Effect.ServerSentEvents -> sse(req.uuid, effect.value.url)
        }
    }

    private fun http(uuid: List<Byte>, method: String, url: String) {
        var builder = Request.Builder().url(url)
        if (method == "POST") {
            builder = builder.method(method, byteArrayOf(0).toRequestBody(null, 0, 0))
        }
        val request = builder.build()

        client.newCall(request)
            .enqueue(
                object : Callback {
                    override fun onFailure(call: Call, e: IOException) {
                        e.printStackTrace()
                    }

                    override fun onResponse(call: Call, response: Response) {
                        response.use {
                            if (!response.isSuccessful)
                                throw IOException("Unexpected code $response")

                            response.body!!.bytes().toList().let { bytes ->
                                update(
                                    CoreMessage.Response(
                                        uuid.toByteArray().toUByteArray().toList(),
                                        Outcome.Http(HttpResponse(response.code.toShort(), bytes))
                                    )
                                )
                            }
                        }
                    }
                }
            )
    }

    private fun sse(uuid: List<Byte>, url: String) {
        val sseClient: OkHttpClient = client.newBuilder()
            .readTimeout(0, TimeUnit.SECONDS)
            .build()

        val request = Request.Builder().url(url).build()
        sseClient.newCall(request)
            .enqueue(
                object : Callback {
                    override fun onFailure(call: Call, e: IOException) {
                        e.printStackTrace()
                    }

                    override fun onResponse(call: Call, response: Response) {
                        response.use {
                            if (!response.isSuccessful)
                                throw IOException("Unexpected code $response")
                            val reader =
                                BufferedReader(InputStreamReader(response.body!!.byteStream()))
                            var line: String
                            while (reader.readLine().also { line = it } != null) {
                                line += "\n\n"
                                update(
                                    CoreMessage.Response(
                                        uuid.toByteArray().toUByteArray().toList(),
                                        Outcome.Sse(Chunk(line.toByteArray().toList()))
                                    )
                                )
                            }
                        }
                    }
                }
            )
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
        Text(text = "Crux Counter Example", fontSize = 30.sp, modifier = Modifier.padding(10.dp))
        Text(text = "Rust Core, Kotlin Shell (Jetpack Compose)", modifier = Modifier.padding(10.dp))
        Text(text = model.view.text, modifier = Modifier.padding(10.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Button(
                onClick = { model.update(CoreMessage.Event(Evt.Decrement())) },
                colors = ButtonDefaults.buttonColors(containerColor = Color.hsl(44F, 1F, 0.77F))
            ) { Text(text = "Decrement", color = Color.DarkGray) }
            Button(
                onClick = { model.update(CoreMessage.Event(Evt.Increment())) },
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
