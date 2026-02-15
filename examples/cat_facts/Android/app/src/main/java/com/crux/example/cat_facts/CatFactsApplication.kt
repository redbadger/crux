package com.crux.example.cat_facts

import android.app.Application
import com.crux.example.cat_facts.di.appModule
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin

class CatFactsApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        startKoin()
    }

    private fun startKoin() {
        startKoin {
            androidContext(this@CatFactsApplication)
            modules(appModule)
        }
    }
}
