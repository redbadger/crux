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
    /// The `HttpHandler` `crux_http` ships, with this app's timeouts. Its
    /// `configure` hook is where a shell's own connection settings go; a shell
    /// that wanted OkHttp or Ktor instead would implement `HttpHandler` over
    /// that and provide it here, and nothing else would change.
    @Provides
    @Singleton
    fun provideHttpHandler(): HttpHandler =
        UrlConnectionHttpHandler { connection ->
            connection.connectTimeout = CONNECT_TIMEOUT_MILLIS
            connection.readTimeout = READ_TIMEOUT_MILLIS
        }

    /// The `KeyValueHandler` `crux_kv` ships, over a directory of the app's
    /// own. A shell that wanted `DataStore` or Room instead would implement
    /// `KeyValueHandler` over that and provide it here, and nothing else would
    /// change.
    @Provides
    @Singleton
    fun provideKeyValueHandler(
        @ApplicationContext context: Context,
    ): KeyValueHandler = FileKeyValueHandler(File(context.filesDir, "key_value_store"))

    /// The `TimeHandler` `crux_time` ships. It owns the timer table, so there
    /// is one of it, for the life of the app.
    @Provides
    @Singleton
    fun provideTimeHandler(): TimeHandler = CoroutineTimeHandler()

    private const val CONNECT_TIMEOUT_MILLIS = 15_000
    private const val READ_TIMEOUT_MILLIS = 30_000
}
