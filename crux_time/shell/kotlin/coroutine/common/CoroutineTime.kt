import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import java.util.concurrent.ConcurrentHashMap
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.Duration.Companion.nanoseconds
import kotlin.time.Duration.Companion.seconds

/**
 * A [TimeHandler] whose timers are coroutines that `delay`.
 *
 * The app's handler holds an instance and delegates each operation to it:
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
 * The timer table is state, so the app constructs one handler where it
 * constructs its own.
 *
 * `CoroutineScope` is written out in full, and `async` used where `launch`
 * would read better, because the generated module's header already imports
 * both of those names above this file's own imports, and Kotlin rejects the
 * same name twice. `kotlin.time.Duration` is written out in full because the
 * generated module has a `Duration` of its own.
 *
 * ## Repeating timers
 *
 * `notifyEvery` runs a coroutine that waits until each deadline and then sends
 * a tick. Each deadline is the start time plus a whole number of periods, on
 * the monotonic `System.nanoTime` clock, so the timer does not drift. If the
 * coroutine wakes after one or more deadlines have passed, it sends one tick
 * and waits for the next deadline that is still ahead.
 *
 * The timer runs until a `clear` for its id arrives. The coroutine checks for
 * the cancel before each tick, but a tick that started before the `clear` can
 * still reach the core after it. The core ignores that tick. If the app aborts
 * the command without a `clear`, the timer continues, and the core ignores its
 * ticks.
 *
 * ## Precision
 *
 * `now` reads `System.currentTimeMillis`, so it has a resolution of one
 * millisecond. `delay` also counts milliseconds, and it rounds a part of a
 * millisecond up, so a timer does not fire early. It can fire late.
 *
 * If [scope] is cancelled, each pending `notifyAt` and `notifyAfter` throws
 * the `CancellationException`, and does not answer. Each repeating timer
 * stops, and sends no more ticks.
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

    /**
     * Send `operation.id` into [sink] once per `operation.period`, until a
     * `clear` for that id arrives.
     *
     * If the app aborts the command without a `clear`, the timer continues,
     * and the core ignores its ticks.
     */
    override fun notifyEvery(
        operation: NotifyEvery,
        sink: EffectSink<TimerId>,
    ) {
        val id = operation.id
        // A period of 0 would make the loop spin.
        val period = clamp(operation.period.nanos).coerceAtLeast(1L)
        // Lazy, as in `sleep`, so that the timer is in the table before it
        // can complete and ask to be taken out again.
        val timer =
            scope.async(start = CoroutineStart.LAZY) {
                val start = System.nanoTime()
                var tick = 1L
                while (true) {
                    val wait = deadline(tick, period) - (System.nanoTime() - start)
                    if (wait > 0L) {
                        delay(wait.nanoseconds)
                        continue
                    }
                    if (!isActive) return@async
                    sink.send(id)
                    // The next deadline that is still ahead, so a late wake
                    // sends one tick, not one for each period it missed.
                    tick = (System.nanoTime() - start) / period + 1L
                }
            }
        timers[id.value] = timer
        timer.invokeOnCompletion { timers.remove(id.value, timer) }
        timer.start()
    }

    override suspend fun clear(operation: ClearTimer): TimerId {
        // Cancelling completes the timer's job, which settles the call waiting
        // on it, so that it answers too. Nothing acts on that answer. A
        // repeating timer checks for the cancel before each tick.
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
     * [tick] periods of [period] nanoseconds, or `Long.MAX_VALUE` if that
     * does not fit.
     */
    private fun deadline(
        tick: Long,
        period: Long,
    ): Long = if (tick > Long.MAX_VALUE / period) Long.MAX_VALUE else tick * period

    /**
     * [count] as a `Long`, or `Long.MAX_VALUE` if it does not fit. Even in
     * nanoseconds, that is more than 292 years, so the timer does not fire
     * while the process runs.
     */
    private fun clamp(count: ULong): Long = count.coerceAtMost(Long.MAX_VALUE.toULong()).toLong()
}
