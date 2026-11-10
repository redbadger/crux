import App
import Foundation

/// Runs `TaskTimeHandler` through the whole `crux_time` protocol and prints
/// `HARNESS OK` when every answer was the one the protocol promises.
///
/// Timing is asserted as ordering and identity, not as precision: a timer is
/// answered with its own id, a cleared one settles rather than waiting out its
/// duration, and a duration is not answered before it has elapsed. Every
/// duration here is tens of milliseconds, except the one that is meant to be
/// cleared long before it fires.

// A hung answer would otherwise wait for the test's own timeout, with nothing
// to say for itself.
Thread.detachNewThread {
    Thread.sleep(forTimeInterval: 60)
    FileHandle.standardError.write(Data("timed out: an operation never answered\n".utf8))
    exit(2)
}

// Every mismatch, so that one run reports all of them rather than the first.
var failures: [String] = []

func expect(_ what: String, _ satisfied: Bool) {
    if !satisfied { failures.append(what) }
}

func expect<T: Equatable>(_ what: String, _ actual: T, _ expected: T) {
    if actual != expected { failures.append("\(what): expected \(expected), got \(actual)") }
}

let time = TaskTimeHandler()

// `now` answers with a wall clock: some time after this source was written,
// and well before the century is out.
let now = await time.now(Now())
expect("now answers with a plausible number of seconds", (1_700_000_000...4_102_444_800).contains(now.seconds))
expect("now answers with a fraction of a second", now.nanos < 1_000_000_000)

// A timer is answered with its own id, once its duration has elapsed.
let started = Date()
let fired = await time.notifyAfter(NotifyAfter(id: TimerId(value: 1), duration: Duration(nanos: 50_000_000)))
let elapsed = Date().timeIntervalSince(started)
expect("notifyAfter answers with the id it was given", fired.value, 1)
expect("notifyAfter waits for the duration it was given, \(elapsed)s", elapsed >= 0.03)

let later = await time.now(Now())
expect(
    "now does not go backwards",
    (later.seconds, later.nanos) >= (now.seconds, now.nanos)
)

// Two at once, each answered with its own id rather than with the other's.
async let two = time.notifyAfter(NotifyAfter(id: TimerId(value: 2), duration: Duration(nanos: 40_000_000)))
async let three = time.notifyAfter(NotifyAfter(id: TimerId(value: 3), duration: Duration(nanos: 10_000_000)))
let (secondFired, thirdFired) = await (two, three)
expect("timers running at once keep their own ids", [secondFired.value, thirdFired.value], [2, 3])

// A timer that is cleared before it fires. Its `notifyAfter` still settles —
// the core has stopped listening by then, so what it answers with does not
// matter, but a call that never returns holds a task for the process's life.
let pending = Task {
    await time.notifyAfter(NotifyAfter(id: TimerId(value: 4), duration: Duration(nanos: 10_000_000_000)))
}
// Long enough for the handler to have the timer in its table before it is
// asked to take it out again.
try? await Task.sleep(nanoseconds: 50_000_000)
let cleared = await time.clear(ClearTimer(id: TimerId(value: 4)))
expect("clear answers with the id it was given", cleared.value, 4)

let waited = Date()
_ = await pending.value
let settling = Date().timeIntervalSince(waited)
expect("a cleared timer's notifyAfter settles rather than waiting out its duration, \(settling)s", settling < 5)

// Clearing a timer that is not there is not an error: the shell need not race
// a timer that has already fired.
let unknown = await time.clear(ClearTimer(id: TimerId(value: 99)))
expect("clear of an unknown timer answers with the id it was given", unknown.value, 99)

if failures.isEmpty {
    print("HARNESS OK")
} else {
    for failure in failures { print("FAILED: \(failure)") }
    exit(1)
}
