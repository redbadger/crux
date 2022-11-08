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
import okhttp3.ResponseBody
import com.redbadger.rmm.shared.*
import com.redbadger.rmm.shared_types.Msg
import com.redbadger.rmm.shared_types.Request as Req
import com.redbadger.rmm.shared_types.RequestBody as ReqBody
import com.redbadger.rmm.shared_types.Response as Res
import com.redbadger.rmm.shared_types.ResponseBody as ResBody
import retrofit2.Call
import retrofit2.Callback
import retrofit2.Response
import retrofit2.Retrofit
import retrofit2.http.GET
import retrofit2.http.Url
import java.time.ZoneOffset
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            AndroidTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background
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
            return Retrofit.Builder().baseUrl("http://dummy.com/").build()
                .create(HttpGetService::class.java)
        }
    }
}

sealed class CoreMessage {
    data class Message(
        val msg: Msg
    ) : CoreMessage()

    data class Response(
        val res: Res
    ) : CoreMessage()
}

class Model : ViewModel() {
    var view: com.redbadger.rmm.shared.ViewModel by mutableStateOf(ViewModel("", null, ""))
        private set

    private val core = Core()

    init {
        update(CoreMessage.Message(Msg.Get()))
        update(CoreMessage.Message(Msg.GetPlatform()))
    }

    @OptIn(ExperimentalUnsignedTypes::class)
    private fun httpGet(url: String, uuid: List<Byte>) {
        val call = HttpGetService.create().get(url)
        call?.enqueue(object : Callback<ResponseBody?> {
            override fun onResponse(
                call: Call<ResponseBody?>?, response: Response<ResponseBody?>?
            ) {
                response?.body()?.bytes()?.toList()?.let { bytes ->
                    update(CoreMessage.Response(Res(uuid, ResBody.Http(bytes))))
                }
            }

            override fun onFailure(call: Call<ResponseBody?>?, t: Throwable?) {}
        })
    }

    fun update(msg: CoreMessage) {
        val requests: List<Req> = when (msg) {
            is CoreMessage.Message -> {
                bcsDeserialize(core.message(msg.msg.bcsSerialize().toUByteArray().toList()))
            }
            is CoreMessage.Response -> {
                bcsDeserialize(core.response(msg.res.bcsSerialize().toUByteArray().toList()))
            }
        }

        for (req in requests) {
            when (val body = req.body) {
                is ReqBody.Render -> {
                    this.view = core.view()
                }
                is ReqBody.Http -> {
                    httpGet(body.value, req.uuid)
                }
                is ReqBody.Time -> {
                    val isoTime =
                        ZonedDateTime.now(ZoneOffset.UTC).format(DateTimeFormatter.ISO_INSTANT)

                    update(CoreMessage.Response(Res(req.uuid, ResBody.Time(isoTime))))
                }
                is ReqBody.Platform -> {
                    val platform = getPlatform()

                    update(CoreMessage.Response(Res(req.uuid, ResBody.Platform(platform))))
                }
                is ReqBody.KVRead -> {
                    update(CoreMessage.Response(Res(req.uuid, ResBody.KVRead(null))))
                }
                is ReqBody.KVWrite -> {
                    update(CoreMessage.Response(Res(req.uuid, ResBody.KVWrite(false))))
                }
            }
        }
    }
}

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
            model.view.image?.let {
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
                onClick = {
                    model.update(CoreMessage.Message(Msg.Clear()))
                },
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
            ) { Text(text = "Clear", color = Color.White) }
            Button(
                onClick = {
                    model.update(CoreMessage.Message(Msg.Get()))
                },
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primary)
            ) { Text(text = "Get", color = Color.White) }
            Button(
                onClick = {
                    model.update(CoreMessage.Message(Msg.Fetch()))
                },
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.secondary)
            ) { Text(text = "Fetch", color = Color.White) }
        }
    }
}

@Preview(showBackground = true)
@Composable
fun DefaultPreview() {
    AndroidTheme { CatFacts() }
}
