package com.example.android

import android.os.Build
import android.os.Bundle
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import okhttp3.ResponseBody
import redbadger.rmm.shared.*
import retrofit2.Call
import retrofit2.Callback
import retrofit2.Response
import retrofit2.Retrofit
import retrofit2.http.GET
import retrofit2.http.Url




class GetPlatform : Platform {
    override fun get(): String {
        return Build.BRAND + " " + Build.VERSION.RELEASE
    }
}

interface CatFactService {
    @GET
    fun getFact(@Url url: String?): Call<ResponseBody?>?

    companion object {

        fun create(): CatFactService {
            return Retrofit.Builder()
                .baseUrl("http://dummy.com/")
                .build()
                .create(CatFactService::class.java)
        }
    }
}

class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val tv1 = findViewById<TextView>(R.id.txt1)
        tv1.text = addForPlatform(1u, 2u, GetPlatform())

        var state = State("");
        update(state, Msg.GetNewFact)?.let { effect ->
            when (effect) {
                is Effect.Get -> {
                    val catFactApi = CatFactService.create().getFact(effect.url);
                    catFactApi?.enqueue(object : Callback<ResponseBody?> {
                        override fun onResponse(
                            call: Call<ResponseBody?>?,
                            response: Response<ResponseBody?>?
                        ) {
                            response?.body()?.bytes()?.toUByteArray()?.toList()?.let { bytes ->
                                update(state, Msg.ReceiveFact(bytes))?.let { effect1 ->
                                    when (effect1) {
                                        is Effect.Render -> {
                                            val tv2 = findViewById<TextView>(R.id.txt2)
                                            tv2.text = effect1.state.catFact
                                        }
                                        else -> {}
                                    }
                                }
                            };
                        }

                        override fun onFailure(call: Call<ResponseBody?>?, t: Throwable?) {}
                    })
                }
                else -> {}
            }

        }
    }
}
