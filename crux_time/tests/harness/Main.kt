import com.example.shared.Clear
import com.example.shared.CoroutineTimeHandler
import com.example.shared.Duration
import com.example.shared.NotifyAfter
import com.example.shared.Now
import com.example.shared.TimerId
import kotlinx.coroutines.async
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import kotlin.system.exitProcess

/// Runs `CoroutineTimeHandler` through the whole `crux_time` protocol and
/// prints `HARNESS OK` when every answer was the one the protocol promises.
///
/// Timing is asserted as ordering and identity, not as precision: a timer is
/// answered with its own id, a cleared one settles rather than waiting out its
/// duration, and a duration is not answered before it has elapsed. Every
/// duration here is tens of milliseconds, except the one that is meant to be
/// cleared long before it fires.

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
        expect("clear answers with the id it was given", time.clear(Clear(TimerId(4uL))).value, 4uL)

        val waited = System.nanoTime()
        pending.await()
        val settling = (System.nanoTime() - waited) / 1_000_000L
        expect(
            "a cleared timer's notifyAfter settles rather than waiting out its duration, ${settling}ms",
            settling < 5_000L,
        )

        // Clearing a timer that is not there is not an error: the shell need
        // not race a timer that has already fired.
        expect("clear of an unknown timer answers with the id it was given", time.clear(Clear(TimerId(99uL))).value, 99uL)

        if (failures.isEmpty()) {
            println("HARNESS OK")
        } else {
            failures.forEach { println("FAILED: $it") }
            exitProcess(1)
        }
    }
