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
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import com.example.counter.shared.*
import com.example.counter.shared_types.*
import com.example.counter.ui.theme.CounterTheme
import okhttp3.*
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException
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
                ) {
                    View()
                }
            }
        }
    }
}

sealed class Outcome {
    data class Http(val res: HttpResponse) : Outcome()
}

sealed class CoreMessage {
    data class Message(val event: Event) : CoreMessage()
    data class Response(val uuid: List<UByte>, val outcome: Outcome) : CoreMessage()
}

class Model : ViewModel() {
    var view: MyViewModel by mutableStateOf(MyViewModel(0))
        private set
    private val client = OkHttpClient()

    init {
        update(CoreMessage.Message(Event.Get()))
    }

    fun update(msg: CoreMessage) {
        val requests: List<Req> =
            when (msg) {
                is CoreMessage.Message -> Requests.bcsDeserialize(
                    message(msg.event.bcsSerialize().toUByteArray().toList()).toUByteArray()
                        .toByteArray()
                )
                is CoreMessage.Response -> Requests.bcsDeserialize(
                    response(
                        msg.uuid.toList(), when (msg.outcome) {
                            is Outcome.Http -> msg.outcome.res.bcsSerialize()
                        }.toUByteArray().toList()
                    ).toUByteArray()
                        .toByteArray()
                )
            }

        for (req in requests) when (val effect = req.effect) {
            is Effect.Render -> {
                this.view = MyViewModel.bcsDeserialize(view().toUByteArray().toByteArray())
            }
            is Effect.Http -> {
                httpRequest(effect.value.method, effect.value.url, req.uuid)
            }

        }
    }

    private fun httpRequest(method: String, url: String, uuid: List<Byte>) {
        var builder = Request.Builder().url(url);
        if (method == "POST") {
            builder = builder.method(method, byteArrayOf(0).toRequestBody(null, 0, 0))

        }
        val request = builder.build()

        client.newCall(request).enqueue(object : Callback {
            override fun onFailure(call: Call, e: IOException) {
                e.printStackTrace()
            }

            override fun onResponse(call: Call, response: Response) {
                response.use {
                    if (!response.isSuccessful) throw IOException("Unexpected code $response")

                    for ((name, value) in response.headers) {
                        println("$name: $value")
                    }

                    response.body!!.bytes().toList()?.let { bytes ->
                        update(
                            CoreMessage.Response(
                                uuid.toByteArray().toUByteArray().toList(),
                                Outcome.Http(HttpResponse(response.code.toShort(), bytes))
                            )
                        )
                    }
                }
            }
        })
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
        Text(text = model.view.count.toString(), modifier = Modifier.padding(10.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Button(
                onClick = { model.update(CoreMessage.Message(Event.Decrement())) },
                colors =
                ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.secondary
                )
            ) { Text(text = "Decrement", color = Color.White) }
            Button(
                onClick = { model.update(CoreMessage.Message(Event.Increment())) },
                colors =
                ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.primary
                )
            ) { Text(text = "Increment", color = Color.White) }
        }
    }
}

@Preview(showBackground = true)
@Composable
fun DefaultPreview() {
    CounterTheme {
        View()
    }
}