package com.crux.examples.counter

import android.app.Application
import com.crux.examples.counter.di.appModule
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
