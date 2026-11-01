package com.crux.example.weather.di

import android.content.Context
import com.crux.example.weather.HttpHandler
import com.crux.example.weather.KeyValueHandler
import com.crux.example.weather.TimeHandler
import com.crux.example.weather.core.DelayTimeHandler
import com.crux.example.weather.core.OkHttpHttpHandler
import com.crux.example.weather.core.PreferencesKeyValueHandler
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import okhttp3.OkHttpClient
import java.util.concurrent.TimeUnit
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
object AppModule {
    @Provides
    @Singleton
    fun provideOkHttpClient(): OkHttpClient =
        OkHttpClient
            .Builder()
            .connectTimeout(15, TimeUnit.SECONDS)
            .readTimeout(30, TimeUnit.SECONDS)
            .writeTimeout(30, TimeUnit.SECONDS)
            .build()

    /** The app's own implementation of the HTTP protocol, over OkHttp. */
    @Provides
    @Singleton
    fun provideHttpHandler(client: OkHttpClient): HttpHandler = OkHttpHttpHandler(client)

    /** The app's own store, in a preferences file of its own. */
    @Provides
    @Singleton
    fun provideKeyValueHandler(
        @ApplicationContext context: Context,
    ): KeyValueHandler =
        PreferencesKeyValueHandler(context.getSharedPreferences("key_value_store", Context.MODE_PRIVATE))

    /** One instance for the life of the app, because it owns the timer table. */
    @Provides
    @Singleton
    fun provideTimeHandler(): TimeHandler = DelayTimeHandler()
}
