package com.example.android

import android.os.Build
import android.os.Bundle
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import redbadger.rmm.shared.Platform
import redbadger.rmm.shared.addForPlatform

class GetPlatform : Platform {
    override fun get(): String {
        return Build.BRAND + " " + Build.VERSION.RELEASE
    }
}

class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val tv = findViewById<TextView>(R.id.txt1)
        tv.text = addForPlatform(1u, 2u, GetPlatform())
    }
}
