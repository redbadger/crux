package com.crux.example.weather.core

import android.content.SharedPreferences
import com.crux.example.weather.BoolResult
import com.crux.example.weather.Delete
import com.crux.example.weather.Exists
import com.crux.example.weather.Get
import com.crux.example.weather.KeyPage
import com.crux.example.weather.KeyValueError
import com.crux.example.weather.KeyValueHandler
import com.crux.example.weather.KeysResult
import com.crux.example.weather.ListKeys
import com.crux.example.weather.Value
import com.crux.example.weather.ValueResult
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.util.Base64
import com.crux.example.weather.Set as KeyValueSet

/**
 * A [KeyValueHandler] over [SharedPreferences]. Each value is stored as a
 * Base64 string, because preferences cannot hold bytes.
 *
 * `SharedPreferences` suits small values such as the favourites, and needs no
 * extra dependency. For larger data, use `DataStore` or Room.
 */
class PreferencesKeyValueHandler(
    private val preferences: SharedPreferences,
) : KeyValueHandler {
    // Off the main thread, because the first read waits for the file to load.
    override suspend fun get(operation: Get): ValueResult =
        withContext(Dispatchers.IO) { ValueResult.Ok(read(operation.key)) }

    override suspend fun set(operation: KeyValueSet): ValueResult =
        withContext(Dispatchers.IO) {
            val previous = read(operation.key)
            val bytes = operation.value.map { it.toByte() }.toByteArray()
            preferences.edit().putString(operation.key, Base64.getEncoder().encodeToString(bytes)).commit()
            ValueResult.Ok(previous)
        }

    override suspend fun delete(operation: Delete): ValueResult =
        withContext(Dispatchers.IO) {
            val previous = read(operation.key)
            preferences.edit().remove(operation.key).commit()
            ValueResult.Ok(previous)
        }

    override suspend fun exists(operation: Exists): BoolResult = BoolResult.Ok(preferences.contains(operation.key))

    override suspend fun listKeys(operation: ListKeys): KeysResult {
        val keys = preferences.all.keys.filter { it.startsWith(operation.prefix) }.sorted()
        if (operation.cursor > keys.size.toULong()) {
            return KeysResult.Err(KeyValueError.CursorNotFound)
        }
        return KeysResult.Ok(KeyPage(keys.drop(operation.cursor.toInt()), 0uL))
    }

    private fun read(key: String): Value {
        val stored = preferences.getString(key, null) ?: return Value.None
        return Value.Bytes(Base64.getDecoder().decode(stored).map { it.toUByte() })
    }
}
