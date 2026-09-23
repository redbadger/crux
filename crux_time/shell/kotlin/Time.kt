import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.delay
import java.util.concurrent.ConcurrentHashMap
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.Duration.Companion.nanoseconds
import kotlin.time.Duration.Companion.seconds

/**
 * The shell side of `crux_time`, as one method per operation.
 *
 * Each method's shape is the generated `EffectHandler`'s, so an app's handler
 * holds an instance and delegates in one line:
 *
 * ```kotlin
 * class MyHandler : EffectHandler {
 *     private val time = CoroutineTimeHandler()
 *
 *     override suspend fun timeNotifyAfter(operation: NotifyAfter): TimerId = time.notifyAfter(operation)
 *     override suspend fun timeClear(operation: ClearTimer): TimerId = time.clear(operation)
 * }
 * ```
 *
 * ## What a timer answers with, and when
 *
 * `notifyAt` and `notifyAfter` are answered exactly once, with the timer's own
 * id, when it fires. `clear` cancels the timer it names and is answered with
 * the same id.
 *
 * A cleared timer's `notifyAt` or `notifyAfter` may still answer — a shell
 * need not race the two. By then the core has stopped listening for that
 * answer and ignores it, which is why the implementation below settles the
 * pending call rather than leaving it suspended for the process's lifetime.
 */
interface TimeHandler {
    /** The current wall-clock time. */
    suspend fun now(operation: Now): Instant

    /** Answer with `operation.id` once `operation.instant` has arrived. */
    suspend fun notifyAt(operation: NotifyAt): TimerId

    /** Answer with `operation.id` once `operation.duration` has elapsed. */
    suspend fun notifyAfter(operation: NotifyAfter): TimerId

    /** Cancel the timer `operation.id` names, and answer with it. */
    suspend fun clear(operation: ClearTimer): TimerId
}

/**
 * A [TimeHandler] whose timers are coroutines that `delay`.
 *
 * The timer table is state, so the app constructs one handler where it
 * constructs its own.
 *
 * `CoroutineScope` is written out in full, and `async` used where `launch`
 * would read better, because the generated module's header already imports
 * both of those names above this file's own imports, and Kotlin rejects the
 * same name twice. `kotlin.time.Duration` is written out in full because the
 * generated module has a `Duration` of its own.
 *
 * ## Precision
 *
 * `now` reads `System.currentTimeMillis`, so it has a resolution of one
 * millisecond. `delay` also counts milliseconds, and it rounds a part of a
 * millisecond up, so a timer does not fire early. It can fire late.
 *
 * If [scope] is cancelled, each pending `notifyAt` and `notifyAfter` throws
 * the `CancellationException`, and does not answer.
 *
 * @param scope the scope the timers run in. The default one outlives every
 *   request, which is what a timer has to do; pass a scope of your own — an
 *   `Activity`'s, a `ViewModel`'s — to tie the timers to its lifetime.
 */
class CoroutineTimeHandler(
    private val scope: kotlinx.coroutines.CoroutineScope =
        kotlinx.coroutines.CoroutineScope(SupervisorJob() + Dispatchers.Default),
) : TimeHandler {
    /** The timers that have not fired or been cleared, by id. */
    private val timers = ConcurrentHashMap<ULong, Job>()

    override suspend fun now(operation: Now): Instant {
        val millis = System.currentTimeMillis()
        return Instant((millis / 1_000L).toULong(), ((millis % 1_000L) * 1_000_000L).toUInt())
    }

    override suspend fun notifyAt(operation: NotifyAt): TimerId {
        val target = clamp(operation.instant.seconds).seconds + operation.instant.nanos.toLong().nanoseconds
        return sleep(operation.id, target - System.currentTimeMillis().milliseconds)
    }

    override suspend fun notifyAfter(operation: NotifyAfter): TimerId =
        sleep(operation.id, clamp(operation.duration.nanos).nanoseconds)

    override suspend fun clear(operation: ClearTimer): TimerId {
        // Cancelling completes the timer's job, which settles the call waiting
        // on it, so that it answers too. Nothing acts on that answer.
        timers.remove(operation.id.value)?.cancel()
        return operation.id
    }

    /**
     * `delay` rounds a part of a millisecond up, so a timer never fires
     * early.
     */
    private suspend fun sleep(
        id: TimerId,
        duration: kotlin.time.Duration,
    ): TimerId {
        val fired = CompletableDeferred<TimerId>()
        // Lazy, so that the timer is in the table before it can complete and
        // ask to be taken out again.
        val timer = scope.async(start = CoroutineStart.LAZY) { delay(duration) }
        timers[id.value] = timer
        timer.invokeOnCompletion { cause ->
            // `clear` takes the timer out of the table before it cancels it, so
            // a timer that is still listed was cancelled by its scope.
            val listed = timers.remove(id.value, timer)
            if (cause is CancellationException && listed) {
                fired.cancel(cause)
            } else {
                fired.complete(id)
            }
        }
        timer.start()

        return fired.await()
    }

    /**
     * [count] as a `Long`, or `Long.MAX_VALUE` if it does not fit. Even in
     * nanoseconds, that is more than 292 years, so the timer does not fire
     * while the process runs.
     */
    private fun clamp(count: ULong): Long = count.coerceAtMost(Long.MAX_VALUE.toULong()).toLong()
}
