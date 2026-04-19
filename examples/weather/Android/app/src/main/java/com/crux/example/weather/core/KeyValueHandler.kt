package com.crux.example.weather.core

import android.util.Log
import com.crux.example.weather.Effect
import com.crux.example.weather.KeyValueOperation
import com.crux.example.weather.KeyValueResponse
import com.crux.example.weather.KeyValueResult
import com.crux.example.weather.Value
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class KeyValueHandler
    @Inject
    constructor(
        private val delegate: KeyValueDataStoreDelegate,
    ) {
        suspend fun handleEffect(effect: Effect.KeyValue): KeyValueResult =
            withContext(Dispatchers.IO) {
                when (val value = effect.value) {
                    is KeyValueOperation.Delete -> delete(value)
                    is KeyValueOperation.Exists -> exists(value)
                    is KeyValueOperation.Get -> get(value)
                    is KeyValueOperation.ListKeys -> listKeys(value)
                    is KeyValueOperation.Set -> set(value)
                }
            }

        private suspend fun delete(value: KeyValueOperation.Delete): KeyValueResult {
            Log.d(TAG, "delete: ${value.key}")
            val deletedValue = delegate.delete(value.key).toValue()
            return KeyValueResult.Ok(KeyValueResponse.Delete(deletedValue))
        }

        private suspend fun exists(value: KeyValueOperation.Exists): KeyValueResult {
            val exists = delegate.exists(value.key)
            Log.d(TAG, "exists: ${value.key} → $exists")
            return KeyValueResult.Ok(KeyValueResponse.Exists(isPresent = exists))
        }

        private suspend fun get(value: KeyValueOperation.Get): KeyValueResult {
            Log.d(TAG, "get: ${value.key}")
            val storedValue = delegate.get(value.key).toValue()
            return KeyValueResult.Ok(KeyValueResponse.Get(storedValue))
        }

        private suspend fun listKeys(value: KeyValueOperation.ListKeys): KeyValueResult {
            Log.d(TAG, "listKeys: prefix=${value.prefix}")
            val keys = delegate.listKeys(value.prefix, value.cursor.toString())
            return KeyValueResult.Ok(KeyValueResponse.ListKeys(keys = keys, nextCursor = 0UL))
        }

        private suspend fun set(value: KeyValueOperation.Set): KeyValueResult {
            val newValue = value.value.decodeToString()
            Log.d(TAG, "set: ${value.key}")
            val previousValue = delegate.set(value.key, newValue).toValue()
            return KeyValueResult.Ok(KeyValueResponse.Set(previousValue))
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
