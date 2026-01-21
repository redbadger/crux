package com.crux.example.weather.utils

import kotlin.coroutines.cancellation.CancellationException

public inline fun <T> runCoroutineCatching(block: () -> T): Result<T> {
    return try {
        Result.success(block())
    } catch (ce: CancellationException) {
        throw ce
    } catch (e: Throwable) {
        Result.failure(e)
    }
}