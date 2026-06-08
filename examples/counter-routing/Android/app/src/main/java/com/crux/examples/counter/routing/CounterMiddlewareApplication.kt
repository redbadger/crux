package com.crux.examples.counter.routing

import android.app.Application
import com.crux.examples.counter.routing.di.appModule
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin

class CounterRoutingApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        startKoin()
    }

    private fun startKoin() {
        startKoin {
            androidContext(this@CounterRoutingApplication)
            modules(appModule)
        }
    }
}
