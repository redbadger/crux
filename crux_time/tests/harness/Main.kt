import com.example.shared.ClearTimer
import com.example.shared.CoroutineTimeHandler
import com.example.shared.Duration
import com.example.shared.EffectHandler
import com.example.shared.EffectSink
import com.example.shared.Instant
import com.example.shared.NotifyAfter
import com.example.shared.NotifyAt
import com.example.shared.NotifyEvery
import com.example.shared.Now
import com.example.shared.TimerId
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import kotlin.system.exitProcess

/**
 * Runs `CoroutineTimeHandler` through the whole `crux_time` protocol and
 * prints `HARNESS OK` when every answer was the one the protocol promises.
 *
 * Timing is asserted as ordering and identity, not as precision: a timer is
 * answered with its own id, a cleared one settles rather than waiting out its
 * duration, and a duration is not answered before it has elapsed. Every
 * duration here is tens of milliseconds, except the one that is meant to be
 * cleared long before it fires.
 */

/**
 * An app's handler, which delegates each operation in one line. It compiles
 * only if each method of `TimeHandler` has the shape of the method of the
 * generated `EffectHandler`.
 */
class Delegating(
    private val time: CoroutineTimeHandler,
) : EffectHandler {
    override suspend fun timeNow(operation: Now): Instant = time.now(operation)

    override suspend fun timeNotifyAt(operation: NotifyAt): TimerId = time.notifyAt(operation)

    override suspend fun timeNotifyAfter(operation: NotifyAfter): TimerId = time.notifyAfter(operation)

    override fun timeNotifyEvery(
        operation: NotifyEvery,
        sink: EffectSink<TimerId>,
    ) = time.notifyEvery(operation, sink)

    override suspend fun timeClear(operation: ClearTimer): TimerId = time.clear(operation)
}

/**
 * The ticks of a repeating timer: the id each one sent, and when, in
 * nanoseconds on the monotonic clock.
 */
class Ticks {
    private val ticks = mutableListOf<Pair<ULong, Long>>()

    val sink = EffectSink<TimerId> { id -> synchronized(ticks) { ticks.add(id.value to System.nanoTime()) } }

    val all: List<Pair<ULong, Long>> get() = synchronized(ticks) { ticks.toList() }
}

// Every mismatch, so that one run reports all of them rather than the first.
val failures = mutableListOf<String>()

fun expect(
    what: String,
    satisfied: Boolean,
) {
    if (!satisfied) failures.add(what)
}

fun <T> expect(
    what: String,
    actual: T,
    expected: T,
) {
    if (actual != expected) failures.add("$what: expected $expected, got $actual")
}

fun main() =
    runBlocking {
        // A hung answer would otherwise wait for the test's own timeout, with
        // nothing to say for itself.
        Thread {
            Thread.sleep(60_000)
            System.err.println("timed out: an operation never answered")
            exitProcess(2)
        }.apply { isDaemon = true }.start()

        val time = CoroutineTimeHandler()

        // `now` answers with a wall clock: some time after this source was
        // written, and well before the century is out.
        val now = time.now(Now)
        expect("now answers with a plausible number of seconds", now.seconds in 1_700_000_000uL..4_102_444_800uL)
        expect("now answers with a fraction of a second", now.nanos < 1_000_000_000u)

        // A timer is answered with its own id, once its duration has elapsed.
        val started = System.nanoTime()
        val fired = time.notifyAfter(NotifyAfter(TimerId(1uL), Duration(50_000_000uL)))
        val elapsed = (System.nanoTime() - started) / 1_000_000L
        expect("notifyAfter answers with the id it was given", fired.value, 1uL)
        expect("notifyAfter waits for the duration it was given, ${elapsed}ms", elapsed >= 30L)

        val later = time.now(Now)
        expect(
            "now does not go backwards",
            later.seconds > now.seconds || (later.seconds == now.seconds && later.nanos >= now.nanos),
        )

        // Two at once, each answered with its own id rather than with the
        // other's.
        val two = async { time.notifyAfter(NotifyAfter(TimerId(2uL), Duration(40_000_000uL))) }
        val three = async { time.notifyAfter(NotifyAfter(TimerId(3uL), Duration(10_000_000uL))) }
        expect("timers running at once keep their own ids", listOf(two.await().value, three.await().value), listOf(2uL, 3uL))

        // A timer that is cleared before it fires. Its `notifyAfter` still
        // settles — the core has stopped listening by then, so what it answers
        // with does not matter, but a call that never returns suspends a
        // coroutine for the process's life.
        val pending = async { time.notifyAfter(NotifyAfter(TimerId(4uL), Duration(10_000_000_000uL))) }
        // Long enough for the handler to have the timer in its table before it
        // is asked to take it out again.
        delay(50)
        expect("clear answers with the id it was given", time.clear(ClearTimer(TimerId(4uL))).value, 4uL)

        val waited = System.nanoTime()
        pending.await()
        val settling = (System.nanoTime() - waited) / 1_000_000L
        expect(
            "a cleared timer's notifyAfter settles rather than waiting out its duration, ${settling}ms",
            settling < 5_000L,
        )

        // Clearing a timer that is not there is not an error: the shell need
        // not race a timer that has already fired.
        expect("clear of an unknown timer answers with the id it was given", time.clear(ClearTimer(TimerId(99uL))).value, 99uL)

        // A timer of less than a millisecond waits for its duration. `delay`
        // takes whole milliseconds, so the handler must round up, not down to 0.
        val short = System.nanoTime()
        time.notifyAfter(NotifyAfter(TimerId(6uL), Duration(900_000uL)))
        val shortElapsed = System.nanoTime() - short
        expect("a 900 µs timer does not fire before its duration, ${shortElapsed}ns", shortElapsed >= 900_000L)

        // When the scope that the timers run in is cancelled, a pending timer
        // throws the cancellation and does not answer with its id. A timer
        // that starts after the cancel throws too.
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        val scoped = CoroutineTimeHandler(scope)
        val orphan = async { runCatching { scoped.notifyAfter(NotifyAfter(TimerId(7uL), Duration(10_000_000_000uL))) } }
        delay(50)
        scope.cancel()
        val orphaned = orphan.await()
        expect(
            "a pending timer throws CancellationException when its scope is cancelled, got $orphaned",
            orphaned.exceptionOrNull() is CancellationException,
        )
        val afterCancel = runCatching { scoped.notifyAfter(NotifyAfter(TimerId(8uL), Duration(10_000_000uL))) }
        expect(
            "a timer started after its scope is cancelled throws CancellationException, got $afterCancel",
            afterCancel.exceptionOrNull() is CancellationException,
        )

        // A repeating timer sends its own id once per period. The `n`th tick
        // comes no earlier than `n` periods after the start, so no tick is
        // early and the ticks never come in a burst.
        val repeating = Ticks()
        val period = 20_000_000L
        val repeatingStart = System.nanoTime()
        Delegating(time).timeNotifyEvery(NotifyEvery(TimerId(10uL), Duration(period.toULong())), repeating.sink)
        delay(130)
        val sent = repeating.all
        expect("notifyEvery sends several ticks, got ${sent.size}", sent.size >= 3)
        expect("notifyEvery sends its own id each time", sent.map { it.first }.toSet(), setOf(10uL))
        expect(
            "notifyEvery does not send a tick before its deadline",
            sent.withIndex().all { (n, tick) -> tick.second - repeatingStart >= (n + 1) * period },
        )

        // `clear` answers with the id, and the timer stops. A tick that
        // started before the `clear` can still arrive, so one more tick is
        // allowed, and then the count must stay the same.
        expect("clear of a repeating timer answers with the id it was given", time.clear(ClearTimer(TimerId(10uL))).value, 10uL)
        val countAtClear = repeating.all.size
        delay(30)
        val countAfterClear = repeating.all.size
        expect("a cleared repeating timer sends at most one more tick", countAfterClear <= countAtClear + 1)
        delay(70)
        expect("a cleared repeating timer sends no more ticks", repeating.all.size, countAfterClear)

        // A one-shot timer and a repeating timer side by side, each with its
        // own id.
        val beside = Ticks()
        time.notifyEvery(NotifyEvery(TimerId(11uL), Duration(10_000_000uL)), beside.sink)
        val oneShot = time.notifyAfter(NotifyAfter(TimerId(12uL), Duration(50_000_000uL)))
        expect("a one-shot timer beside a repeating one answers with its own id", oneShot.value, 12uL)
        val besideTicks = beside.all
        expect("a repeating timer beside a one-shot one ticks, got ${besideTicks.size}", besideTicks.size >= 2)
        expect("a repeating timer beside a one-shot one sends its own id", besideTicks.map { it.first }.toSet(), setOf(11uL))
        time.clear(ClearTimer(TimerId(11uL)))
        delay(20)
        val besideAtClear = beside.all.size
        delay(40)
        expect("the repeating timer beside a one-shot one stops when it is cleared", beside.all.size, besideAtClear)

        if (failures.isEmpty()) {
            println("HARNESS OK")
        } else {
            failures.forEach { println("FAILED: $it") }
            exitProcess(1)
        }
    }
