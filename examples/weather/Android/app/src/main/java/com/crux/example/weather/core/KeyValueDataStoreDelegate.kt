package com.crux.example.weather.core

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.first

class KeyValueDataStoreDelegate(context: Context) {
    private val Context.keyValueDataStore: DataStore<Preferences> by preferencesDataStore(
        name = PREFS_NAME
    )
    private val dataStore = context.keyValueDataStore

    suspend fun get(key: String): String? {
        val preferenceKey = stringPreferencesKey(key)
        return dataStore.data.first()[preferenceKey]
    }

    suspend fun set(key: String, value: String): String? {
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

    suspend fun listKeys(prefix: String, cursor: String?): List<String> {
        val keys = dataStore.data.first()
            .asMap()
            .keys
            .asSequence()

        return keys
            .map { it.name }
            .filter { it.startsWith(prefix) }
            .filter { cursor.isNullOrEmpty() || it > cursor }
            .sorted()
            .toList()
    }

    companion object {
        private const val PREFS_NAME = "key_value_store"
    }
}
