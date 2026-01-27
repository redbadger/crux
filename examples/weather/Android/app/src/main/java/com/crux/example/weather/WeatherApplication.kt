package com.crux.example.weather

import android.app.Application
import android.system.Os
import com.crux.example.weather.di.appModule
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin

class WeatherApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        setOpenWeatherApiKey()
        startKoin()
    }

    private fun startKoin() {
        startKoin {
            androidContext(this@WeatherApplication)
            modules(appModule)
        }
    }

    private fun setOpenWeatherApiKey() {
        Os.setenv("OPENWEATHER_API_KEY", BuildConfig.OPENWEATHER_API_KEY, true)
    }
}
