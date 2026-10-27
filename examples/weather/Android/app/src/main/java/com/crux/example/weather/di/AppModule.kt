package com.crux.example.weather.di

import android.content.Context
import com.crux.example.weather.CoroutineTimeHandler
import com.crux.example.weather.FileKeyValueHandler
import com.crux.example.weather.HttpHandler
import com.crux.example.weather.KeyValueHandler
import com.crux.example.weather.TimeHandler
import com.crux.example.weather.UrlConnectionHttpHandler
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import java.io.File
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
object AppModule {
    /**
     * The handler that `crux_http` ships, with this app's timeouts. To use OkHttp
     * or Ktor, implement `HttpHandler` and provide it here.
     */
    @Provides
    @Singleton
    fun provideHttpHandler(): HttpHandler =
        UrlConnectionHttpHandler { connection ->
            connection.connectTimeout = CONNECT_TIMEOUT_MILLIS
            connection.readTimeout = READ_TIMEOUT_MILLIS
        }

    /**
     * The handler that `crux_kv` ships, over a directory of the app's own. To use
     * `DataStore` or Room, implement `KeyValueHandler` and provide it here.
     */
    @Provides
    @Singleton
    fun provideKeyValueHandler(
        @ApplicationContext context: Context,
    ): KeyValueHandler = FileKeyValueHandler(File(context.filesDir, "key_value_store"))

    /** One instance for the life of the app, because it owns the timer table. */
    @Provides
    @Singleton
    fun provideTimeHandler(): TimeHandler = CoroutineTimeHandler()

    private const val CONNECT_TIMEOUT_MILLIS = 15_000
    private const val READ_TIMEOUT_MILLIS = 30_000
}
