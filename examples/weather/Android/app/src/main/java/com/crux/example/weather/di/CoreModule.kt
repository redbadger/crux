package com.crux.example.weather.di

import com.crux.example.weather.Core
import com.crux.example.weather.Event
import com.crux.example.weather.core.WeatherHandler
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import javax.inject.Singleton

/// `Core` is generated, so it has no `@Inject` constructor for Hilt to find —
/// this module builds it instead. The bridge over the Rust core is generated
/// too, so nothing here mentions the FFI.
@Module
@InstallIn(SingletonComponent::class)
object CoreModule {
    // ANCHOR: start
    @Provides
    @Singleton
    fun provideCore(handler: WeatherHandler): Core =
        Core(
            handler,
            CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate),
        ).also { it.update(Event.Start) }
    // ANCHOR_END: start
}
