package com.crux.examples.counter.middleware

import android.app.Application
import com.crux.examples.counter.middleware.di.appModule
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin

class CounterMiddlewareApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        startKoin()
    }

    private fun startKoin() {
        startKoin {
            androidContext(this@CounterMiddlewareApplication)
            modules(appModule)
        }
    }
}
