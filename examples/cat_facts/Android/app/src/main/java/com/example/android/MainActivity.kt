@file:OptIn(ExperimentalUnsignedTypes::class)

package com.example.android

import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Public
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import coil.compose.rememberAsyncImagePainter
import com.example.android.ui.theme.AndroidTheme
import com.redbadger.catfacts.shared.*
import com.redbadger.catfacts.shared_types.*
import okhttp3.ResponseBody
import retrofit2.Call
import retrofit2.Callback
import retrofit2.Response
import retrofit2.Retrofit
import retrofit2.http.GET
import retrofit2.http.Url
import java.time.ZoneOffset
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import java.util.*
import kotlin.jvm.optionals.getOrNull
import com.redbadger.catfacts.shared_types.Request as Req
import com.redbadger.catfacts.shared_types.ViewModel as MyViewModel

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            AndroidTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) { CatFacts() }
            }
        }
    }
}

fun getPlatform(): String {
    return Build.BRAND + " " + Build.VERSION.RELEASE
}

interface HttpGetService {
    @GET
    fun get(@Url url: String?): Call<ResponseBody?>?

    companion object {
        fun create(): HttpGetService {
            return Retrofit.Builder()
                .baseUrl("http://dummy.com/")
                .build()
                .create(HttpGetService::class.java)
        }
    }
}

sealed class Outcome {
    data class Platform(val res: PlatformResponse) : Outcome()
    data class Time(val res: TimeResponse) : Outcome()
    data class Http(val res: HttpResponse) : Outcome()
    data class KeyValue(val res: KeyValueResponse) : Outcome()
}

sealed class CoreMessage {
    data class Message(val event: Event) : CoreMessage()
    data class Response(val uuid: List<UByte>, val outcome: Outcome) : CoreMessage()
}

class Model : ViewModel() {
    var view: MyViewModel by mutableStateOf(MyViewModel("", Optional.empty(), ""))
        private set

    init {
        update(CoreMessage.Message(Event.Get()))
        update(CoreMessage.Message(Event.GetPlatform()))
    }

    private fun httpGet(url: String, uuid: List<Byte>) {
        val call = HttpGetService.create().get(url)
        call?.enqueue(
            object : Callback<ResponseBody?> {
                override fun onResponse(
                    call: Call<ResponseBody?>?,
                    response: Response<ResponseBody?>?
                ) {
                    response?.body()?.bytes()?.toList()?.let { bytes ->
                        update(
                            CoreMessage.Response(
                                uuid.toByteArray().toUByteArray().toList(),
                                Outcome.Http(HttpResponse(response.code().toShort(), bytes))
                            )
                        )
                    }
                }

                override fun onFailure(call: Call<ResponseBody?>?, t: Throwable?) {}
            }
        )
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
                            is Outcome.Platform -> msg.outcome.res.bcsSerialize()
                            is Outcome.Time -> msg.outcome.res.bcsSerialize()
                            is Outcome.Http -> msg.outcome.res.bcsSerialize()
                            is Outcome.KeyValue -> msg.outcome.res.bcsSerialize()
                        }.toUByteArray().toList()
                    ).toUByteArray()
                        .toByteArray()
                )
            }

        for (req in requests) when (val effect =     req.effect) {
            is Effect.Render -> {
                this.view = MyViewModel.bcsDeserialize(view().toUByteArray().toByteArray())
            }
            is Effect.Http -> {
                httpGet(effect.value.url, req.uuid)
            }
            is Effect.Time -> {
                val isoTime =
                    ZonedDateTime.now(ZoneOffset.UTC).format(DateTimeFormatter.ISO_INSTANT)

                update(
                    CoreMessage.Response(
                        req.uuid.toByteArray().toUByteArray().toList(), Outcome.Time(
                            TimeResponse(isoTime)
                        )
                    )
                )
            }
            is Effect.Platform -> {
                val platform = getPlatform()

                update(
                    CoreMessage.Response(
                        req.uuid.toByteArray().toUByteArray().toList(), Outcome.Platform(
                            PlatformResponse(platform)
                        )
                    )
                )
            }
            is Effect.KeyValue -> when (effect.value) {
                is KeyValueRequest.Read -> update(
                    CoreMessage.Response(
                        req.uuid.toByteArray().toUByteArray().toList(),
                        Outcome.KeyValue(KeyValueResponse.Read(null))
                    )
                )
                is KeyValueRequest.Write -> update(
                    CoreMessage.Response(
                        req.uuid.toByteArray().toUByteArray().toList(),
                        Outcome.KeyValue(KeyValueResponse.Write(false)),
                    )
                )
            }
        }
    }
}

@OptIn(ExperimentalStdlibApi::class)
@Composable
fun CatFacts(model: Model = viewModel()) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
        modifier = Modifier
            .fillMaxSize()
            .padding(10.dp),
    ) {
        Icon(Icons.Filled.Public, "Platform")
        Text(text = model.view.platform, modifier = Modifier.padding(10.dp))
        Row(
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier
                .height(250.dp)
                .padding(10.dp)
        ) {
            model.view.image.getOrNull()?.let {
                Image(
                    painter = rememberAsyncImagePainter(it.file),
                    contentDescription = "cat image",
                    modifier = Modifier
                        .height(250.dp)
                        .fillMaxWidth()
                )
            }
        }
        Text(text = model.view.fact, modifier = Modifier.padding(10.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Button(
                onClick = { model.update(CoreMessage.Message(Event.Clear())) },
                colors =
                ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.error
                )
            ) { Text(text = "Clear", color = Color.White) }
            Button(
                onClick = { model.update(CoreMessage.Message(Event.Get())) },
                colors =
                ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.primary
                )
            ) { Text(text = "Get", color = Color.White) }
            Button(
                onClick = { model.update(CoreMessage.Message(Event.Fetch())) },
                colors =
                ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.secondary
                )
            ) { Text(text = "Fetch", color = Color.White) }
        }
    }
}

@Preview(showBackground = true)
@Composable
fun DefaultPreview() {
    AndroidTheme { CatFacts() }
}
