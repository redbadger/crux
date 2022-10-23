package com.example.android

import android.os.Build
import android.os.Bundle
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import redbadger.rmm.shared.*

class GetPlatform : Platform {
    override fun get(): String {
        return Build.BRAND + " " + Build.VERSION.RELEASE
    }
}

class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val tv1 = findViewById<TextView>(R.id.txt1)
        tv1.text = addForPlatform(1u, 2u, GetPlatform())

        val tv2 = findViewById<TextView>(R.id.txt2)
        tv2.text = catFact()
    }
}
