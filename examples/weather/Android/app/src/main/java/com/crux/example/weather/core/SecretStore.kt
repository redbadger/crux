package com.crux.example.weather.core

import android.content.Context
import android.content.SharedPreferences
import android.util.Log
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.crux.example.weather.SecretRequest
import com.crux.example.weather.SecretResponse
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

        suspend fun handle(request: SecretRequest): SecretResponse =
            withContext(Dispatchers.IO) {
                Log.d(TAG, "handle: $request")
                when (request) {
                    is SecretRequest.Fetch -> fetch(request.value)
                    is SecretRequest.Store -> store(request.field0, request.field1)
                    is SecretRequest.Delete -> delete(request.value)
                }
            }

        private fun fetch(key: String): SecretResponse {
            val value = prefs.getString(key, null)
            return if (value != null) {
                Log.d(TAG, "fetched secret for key: $key")
                SecretResponse.Fetched(key, value)
            } else {
                Log.d(TAG, "no secret found for key: $key")
                SecretResponse.Missing(key)
            }
        }

        private fun store(
            key: String,
            value: String,
        ): SecretResponse =
            try {
                prefs.edit().putString(key, value).apply()
                Log.d(TAG, "stored secret for key: $key")
                SecretResponse.Stored(key)
            } catch (e: Exception) {
                Log.w(TAG, "failed to store secret for key $key: ${e.message}")
                SecretResponse.StoreError(e.message ?: "Unknown error")
            }

        private fun delete(key: String): SecretResponse =
            try {
                prefs.edit().remove(key).apply()
                Log.d(TAG, "deleted secret for key: $key")
                SecretResponse.Deleted(key)
            } catch (e: Exception) {
                Log.w(TAG, "failed to delete secret for key $key: ${e.message}")
                SecretResponse.DeleteError(e.message ?: "Unknown error")
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
