package com.example.android

import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
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
import com.example.android.ui.theme.AndroidTheme
import okhttp3.ResponseBody
import redbadger.rmm.shared.*
import retrofit2.Call
import retrofit2.Callback
import retrofit2.Response
import retrofit2.Retrofit
import retrofit2.http.GET
import retrofit2.http.Url

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

class GetPlatform : Platform {
    override fun get(): String {
        return Build.BRAND + " " + Build.VERSION.RELEASE
    }
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

class Model : ViewModel() {
    var catFact: String by mutableStateOf("Initial")
        private set

    private val core = Core()

    init {
        update(Msg.GetFact)
    }

    private fun getFact(url: String) {
        val catFactApi = HttpGetService.create().get(url);
        catFactApi?.enqueue(object : Callback<ResponseBody?> {
            override fun onResponse(
                call: Call<ResponseBody?>?, response: Response<ResponseBody?>?
            ) {
                response?.body()?.bytes()?.toUByteArray()?.toList()?.let { bytes ->
                    update(Msg.ReceiveFact(bytes))
                }
            }

            override fun onFailure(call: Call<ResponseBody?>?, t: Throwable?) {}
        })
    }

    fun update(msg: Msg) {
        when (val cmd = core.update(msg)) {
            is Cmd.Render -> {
                this.catFact = cmd.catFact
            }
            is Cmd.Get -> {
                getFact(cmd.url)
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
        Text(text = addForPlatform(1u, 2u, GetPlatform()), modifier = Modifier.padding(10.dp))
        Text(text = model.catFact, modifier = Modifier.padding(10.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Button(
                onClick = {
                    model.update(Msg.ClearFact)
                },
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
            ) { Text(text = "Clear", color = Color.White) }
            Button(
                onClick = {
                    model.update(Msg.GetFact)
                },
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primary)
            ) { Text(text = "Get", color = Color.White) }
            Button(
                onClick = {
                    model.update(Msg.FetchFact)
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
