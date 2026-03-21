package com.crux.examples.counter.http

import android.app.Application
import com.crux.examples.counter.http.di.appModule
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin

class CounterApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        startKoin()
    }

    private fun startKoin() {
        startKoin {
            androidContext(this@CounterApplication)
            modules(appModule)
        }
    }
}
