package com.crux.example.weather

import android.app.Application
import com.crux.example.weather.di.appModule
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin

class WeatherApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        startKoin()
    }

    private fun startKoin() {
        startKoin {
            androidContext(this@WeatherApplication)
            modules(appModule)
        }
    }
}
