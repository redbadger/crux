package com.crux.examples.notes

import android.util.Base64
import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.firstOrNull
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch

open class Core(private val dataStore: DataStore<Preferences>) : androidx.lifecycle.ViewModel() {
    private var core: CoreFfi = CoreFfi()
    private val timerJobs = mutableMapOf<ULong, Job>()

    var view: ViewModel by mutableStateOf(
        ViewModel.bincodeDeserialize(core.view())
    )
        private set

    fun update(event: Event) {
        val effects = core.update(event.bincodeSerialize())

        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processEffect(request)
        }
    }

    private fun processEffect(request: Request) {
        when (val effect = request.effect) {
            is Effect.Render -> {
                this.view = ViewModel.bincodeDeserialize(core.view())
            }
            is Effect.KeyValue -> {
                handleKeyValue(request.id, effect.value)
            }
            is Effect.Time -> {
                handleTime(request.id, effect.value)
            }
            is Effect.PubSub -> {
                when (val op = effect.value) {
                    is PubSubOperation.Publish -> {
                        Log.d("Notes", "PubSub publish: ${op.value.size} bytes")
                    }
                    is PubSubOperation.Subscribe -> {
                        Log.d("Notes", "PubSub subscribe (no backend)")
                    }
                }
            }
        }
    }

    private fun resolveEffects(requestId: UInt, data: ByteArray) {
        val effects = core.resolve(requestId, data)
        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processEffect(request)
        }
    }

    private fun handleKeyValue(requestId: UInt, operation: KeyValueOperation) {
        viewModelScope.launch {
            val result = processKeyValueOperation(operation)
            resolveEffects(requestId, result.bincodeSerialize())
        }
    }

    private suspend fun processKeyValueOperation(operation: KeyValueOperation): KeyValueResult {
        return when (operation) {
            is KeyValueOperation.Get -> {
                val key = stringPreferencesKey(operation.key)
                val encoded = dataStore.data.map { it[key] }.firstOrNull()
                val bytes = if (encoded != null) {
                    Base64.decode(encoded, Base64.DEFAULT)
                } else {
                    ByteArray(0)
                }
                KeyValueResult.Ok(KeyValueResponse.Get(Value.Bytes(bytes.toList())))
            }
            is KeyValueOperation.Set -> {
                val key = stringPreferencesKey(operation.key)
                val previousEncoded = dataStore.data.map { it[key] }.firstOrNull()
                val previousBytes = if (previousEncoded != null) {
                    Base64.decode(previousEncoded, Base64.DEFAULT)
                } else {
                    ByteArray(0)
                }
                val encoded = Base64.encodeToString(
                    operation.value.toByteArray(), Base64.DEFAULT
                )
                dataStore.edit { it[key] = encoded }
                KeyValueResult.Ok(KeyValueResponse.Set(Value.Bytes(previousBytes.toList())))
            }
            is KeyValueOperation.Delete -> {
                val key = stringPreferencesKey(operation.key)
                val previousEncoded = dataStore.data.map { it[key] }.firstOrNull()
                val previousBytes = if (previousEncoded != null) {
                    Base64.decode(previousEncoded, Base64.DEFAULT)
                } else {
                    ByteArray(0)
                }
                dataStore.edit { it.remove(key) }
                KeyValueResult.Ok(KeyValueResponse.Delete(Value.Bytes(previousBytes.toList())))
            }
            is KeyValueOperation.Exists -> {
                val key = stringPreferencesKey(operation.key)
                val exists = dataStore.data.map { it.contains(key) }.firstOrNull() ?: false
                KeyValueResult.Ok(KeyValueResponse.Exists(exists))
            }
            is KeyValueOperation.ListKeys -> {
                val allKeys = dataStore.data.map { prefs ->
                    prefs.asMap().keys
                        .map { it.name }
                        .filter { it.startsWith(operation.prefix) }
                }.firstOrNull() ?: emptyList()
                KeyValueResult.Ok(KeyValueResponse.ListKeys(allKeys, 0UL))
            }
        }
    }

    private fun handleTime(requestId: UInt, operation: TimeRequest) {
        when (operation) {
            is TimeRequest.NotifyAfter -> {
                val timerId = operation.id
                val durationMs = operation.duration.nanos / 1_000_000UL

                val job = viewModelScope.launch {
                    delay(durationMs.toLong())
                    val response = TimeResponse.DurationElapsed(timerId)
                    resolveEffects(requestId, response.bincodeSerialize())
                }
                timerJobs[timerId] = job
            }
            is TimeRequest.Clear -> {
                timerJobs[operation.id]?.cancel()
                timerJobs.remove(operation.id)
            }
            else -> {
                Log.w("Notes", "Unhandled time request: $operation")
            }
        }
    }
}
