package com.crux.example.weather.core

import android.util.Log
import com.crux.example.weather.Get
import com.crux.example.weather.Value
import com.crux.example.weather.ValueResult
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import javax.inject.Inject
import javax.inject.Singleton
import com.crux.example.weather.Set as KeyValueSet

@Singleton
class KeyValueHandler
    @Inject
    constructor(
        private val delegate: KeyValueDataStoreDelegate,
    ) {
        suspend fun get(operation: Get): ValueResult =
            withContext(Dispatchers.IO) {
                Log.d(TAG, "get: ${operation.key}")
                ValueResult.Ok(delegate.get(operation.key).toValue())
            }

        suspend fun set(operation: KeyValueSet): ValueResult =
            withContext(Dispatchers.IO) {
                Log.d(TAG, "set: ${operation.key}")
                val newValue = operation.value.decodeToString()
                ValueResult.Ok(delegate.set(operation.key, newValue).toValue())
            }

        private fun List<UByte>.decodeToString(): String = String(this.map { it.toByte() }.toByteArray())

        private fun String?.toValue(): Value {
            val bytes = this?.toByteArray()?.map { it.toUByte() }
            return if (bytes.isNullOrEmpty()) {
                Value.None
            } else {
                Value.Bytes(bytes)
            }
        }

        companion object {
            private const val TAG = "KeyValueHandler"
        }
    }
