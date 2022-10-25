package com.example.android

import android.os.Build
import android.os.Bundle
import android.util.Log
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import redbadger.rmm.shared.*
import retrofit2.Call
import retrofit2.Callback
import retrofit2.Response
import retrofit2.Retrofit
import retrofit2.converter.gson.GsonConverterFactory
import retrofit2.http.GET

class GetPlatform : Platform {
    override fun get(): String {
        return Build.BRAND + " " + Build.VERSION.RELEASE
    }
}

interface CatFactService {
    @GET("fact")
    fun getFact(): Call<CatFactData?>?

    companion object {
        private const val baseUrl = "https://catfact.ninja/"

        fun create(): CatFactService {
            return Retrofit.Builder()
                .baseUrl(baseUrl)
                .addConverterFactory(GsonConverterFactory.create())
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

        val catFactApi = CatFactService.create().getFact();
        catFactApi?.enqueue(object: Callback<CatFactData?> {
            override fun onResponse(call: Call<CatFactData?>?, response: Response<CatFactData?>?) {
                if (response?.body() != null) {
                    Log.d("result: ", response.body().toString())
                    val tv2 = findViewById<TextView>(R.id.txt2)
                    tv2.text = response.body()?.let { CatFact(it).format() }
                }
            }

            override fun onFailure(call: Call<CatFactData?>?, t: Throwable?) {

            }

        })

    }
}
