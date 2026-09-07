package com.crux.example.weather.di

import com.crux.example.weather.Core
import com.crux.example.weather.CoreBridge
import com.crux.example.weather.Event
import com.crux.example.weather.core.LiveBridge
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
/// this module builds it instead, and binds the bridge it talks through.
@Module
@InstallIn(SingletonComponent::class)
object CoreModule {
    @Provides
    @Singleton
    fun provideCoreBridge(bridge: LiveBridge): CoreBridge = bridge

    // ANCHOR: start
    @Provides
    @Singleton
    fun provideCore(
        bridge: CoreBridge,
        handler: WeatherHandler,
    ): Core =
        Core(
            bridge,
            handler,
            CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate),
        ).also { it.update(Event.Start) }
    // ANCHOR_END: start
}
