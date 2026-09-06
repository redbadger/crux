package com.crux.example.weather.core

import android.content.Context
import android.content.SharedPreferences
import android.util.Log
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.crux.example.weather.SecretDeleteResponse
import com.crux.example.weather.SecretFetchResponse
import com.crux.example.weather.SecretStoreResponse
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class SecretStore
    @Inject
    constructor(
        @ApplicationContext context: Context,
    ) {
        // Lazy: EncryptedSharedPreferences.create() is blocking I/O (Keystore access).
        // Defer until first use so Hilt construction stays off the main thread.
        private val prefs: SharedPreferences by lazy { createEncryptedPrefs(context) }

        // Each operation has its own response type, naming only the outcomes it
        // can actually produce — no wide response enum to narrow.
        suspend fun fetch(key: String): SecretFetchResponse =
            withContext(Dispatchers.IO) {
                val value = prefs.getString(key, null)
                if (value != null) {
                    Log.d(TAG, "fetched secret for key: $key")
                    SecretFetchResponse.Fetched(value)
                } else {
                    Log.d(TAG, "no secret found for key: $key")
                    SecretFetchResponse.Missing(key)
                }
            }

        suspend fun store(
            key: String,
            value: String,
        ): SecretStoreResponse =
            withContext(Dispatchers.IO) {
                try {
                    prefs.edit().putString(key, value).apply()
                    Log.d(TAG, "stored secret for key: $key")
                    SecretStoreResponse.Stored(key)
                } catch (e: Exception) {
                    Log.w(TAG, "failed to store secret for key $key: ${e.message}")
                    SecretStoreResponse.StoreError(e.message ?: "Unknown error")
                }
            }

        suspend fun delete(key: String): SecretDeleteResponse =
            withContext(Dispatchers.IO) {
                try {
                    prefs.edit().remove(key).apply()
                    Log.d(TAG, "deleted secret for key: $key")
                    SecretDeleteResponse.Deleted(key)
                } catch (e: Exception) {
                    Log.w(TAG, "failed to delete secret for key $key: ${e.message}")
                    SecretDeleteResponse.DeleteError(e.message ?: "Unknown error")
                }
            }

        companion object {
            private const val TAG = "SecretStore"
            private const val PREFS_FILE = "crux_weather_secrets"

            private fun createEncryptedPrefs(context: Context): SharedPreferences {
                val masterKey =
                    MasterKey
                        .Builder(context)
                        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                        .build()
                return EncryptedSharedPreferences.create(
                    context,
                    PREFS_FILE,
                    masterKey,
                    EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                    EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
                )
            }
        }
    }
