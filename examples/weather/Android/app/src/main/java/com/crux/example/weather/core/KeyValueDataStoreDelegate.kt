package com.crux.example.weather.core

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.flow.first
import javax.inject.Inject
import javax.inject.Singleton

private val Context.keyValueDataStore: DataStore<Preferences> by preferencesDataStore(
    name = "key_value_store",
)

@Singleton
class KeyValueDataStoreDelegate
    @Inject
    constructor(
        @ApplicationContext context: Context,
    ) {
        private val dataStore = context.keyValueDataStore

        suspend fun get(key: String): String? {
            val preferenceKey = stringPreferencesKey(key)
            return dataStore.data.first()[preferenceKey]
        }

        suspend fun set(
            key: String,
            value: String,
        ): String? {
            val preferenceKey = stringPreferencesKey(key)
            val previousValue = dataStore.data.first()[preferenceKey]
            dataStore.edit { prefs ->
                prefs[preferenceKey] = value
            }
            return previousValue
        }

        suspend fun delete(key: String): String? {
            val preferenceKey = stringPreferencesKey(key)
            val deletedValue = dataStore.data.first()[preferenceKey]
            dataStore.edit { prefs ->
                prefs.remove(preferenceKey)
            }
            return deletedValue
        }

        suspend fun exists(key: String): Boolean {
            val preferenceKey = stringPreferencesKey(key)
            return dataStore.data.first().contains(preferenceKey)
        }

        suspend fun listKeys(
            prefix: String,
            cursor: String?,
        ): List<String> =
            dataStore.data
                .first()
                .asMap()
                .keys
                .asSequence()
                .map { it.name }
                .filter { it.startsWith(prefix) }
                .filter { cursor.isNullOrEmpty() || it > cursor }
                .sorted()
                .toList()
    }
