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

/// An app's handler, which delegates each operation in one line. It compiles
/// only if each method of `TimeHandler` has the shape of the method of the
/// generated `EffectHandler`.
struct Delegating: EffectHandler {
    let time: TaskTimeHandler

    func timeNow(_ operation: Now) async -> Instant { await time.now(operation) }
    func timeNotifyAt(_ operation: NotifyAt) async -> TimerId { await time.notifyAt(operation) }
    func timeNotifyAfter(_ operation: NotifyAfter) async -> TimerId { await time.notifyAfter(operation) }
    func timeNotifyEvery(_ operation: NotifyEvery, into sink: EffectSink<TimerId>) { time.notifyEvery(operation, into: sink) }
    func timeClear(_ operation: ClearTimer) async -> TimerId { await time.clear(operation) }
}

/// The ticks of a repeating timer: the id each one sent, and when, in
/// nanoseconds on the monotonic clock.
final class Ticks: @unchecked Sendable {
    private let lock = NSLock()
    private var ticks: [(id: UInt64, at: UInt64)] = []

    var sink: EffectSink<TimerId> {
        EffectSink { id in self.record(id.value) }
    }

    func record(_ id: UInt64) {
        lock.lock()
        defer { lock.unlock() }
        ticks.append((id, DispatchTime.now().uptimeNanoseconds))
    }

    var all: [(id: UInt64, at: UInt64)] {
        lock.lock()
        defer { lock.unlock() }
        return ticks
    }
}

func pause(_ millis: UInt64) async {
    try? await Task.sleep(nanoseconds: millis * 1_000_000)
}

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

// A repeating timer sends its own id once per period. The `n`th tick comes no
// earlier than `n` periods after the start, so no tick is early and the ticks
// never come in a burst.
let repeating = Ticks()
let period: UInt64 = 20_000_000
let repeatingStart = DispatchTime.now().uptimeNanoseconds
Delegating(time: time).timeNotifyEvery(
    NotifyEvery(id: TimerId(value: 10), period: Duration(nanos: period)),
    into: repeating.sink
)
await pause(130)
let sent = repeating.all
expect("notifyEvery sends several ticks, got \(sent.count)", sent.count >= 3)
expect("notifyEvery sends its own id each time", Set(sent.map(\.id)), [10])
expect(
    "notifyEvery does not send a tick before its deadline",
    sent.enumerated().allSatisfy { n, tick in tick.at - repeatingStart >= UInt64(n + 1) * period }
)

// `clear` answers with the id, and the timer stops. A tick that started
// before the `clear` can still arrive, so one more tick is allowed, and then
// the count must stay the same.
let clearedRepeating = await time.clear(ClearTimer(id: TimerId(value: 10)))
expect("clear of a repeating timer answers with the id it was given", clearedRepeating.value, 10)
let countAtClear = repeating.all.count
await pause(30)
let countAfterClear = repeating.all.count
expect("a cleared repeating timer sends at most one more tick", countAfterClear <= countAtClear + 1)
await pause(70)
expect("a cleared repeating timer sends no more ticks", repeating.all.count, countAfterClear)

// A one-shot timer and a repeating timer side by side, each with its own id.
let beside = Ticks()
time.notifyEvery(NotifyEvery(id: TimerId(value: 11), period: Duration(nanos: 10_000_000)), into: beside.sink)
let oneShot = await time.notifyAfter(NotifyAfter(id: TimerId(value: 12), duration: Duration(nanos: 50_000_000)))
expect("a one-shot timer beside a repeating one answers with its own id", oneShot.value, 12)
let besideTicks = beside.all
expect("a repeating timer beside a one-shot one ticks, got \(besideTicks.count)", besideTicks.count >= 2)
expect("a repeating timer beside a one-shot one sends its own id", Set(besideTicks.map(\.id)), [11])
_ = await time.clear(ClearTimer(id: TimerId(value: 11)))
await pause(20)
let besideAtClear = beside.all.count
await pause(40)
expect("the repeating timer beside a one-shot one stops when it is cleared", beside.all.count, besideAtClear)

if failures.isEmpty {
    print("HARNESS OK")
} else {
    for failure in failures { print("FAILED: \(failure)") }
    exit(1)
}
