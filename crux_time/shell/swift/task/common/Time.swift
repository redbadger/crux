import Foundation

/// The shell side of `crux_time`, as one method per operation.
///
/// Each method's shape is the generated `EffectHandler`'s, so an app's handler
/// holds an instance and delegates in one line:
///
/// ```swift
/// struct MyHandler: EffectHandler {
///     let time = TaskTimeHandler()
///
///     func timeNotifyAfter(_ operation: NotifyAfter) async -> TimerId { await time.notifyAfter(operation) }
///     func timeClear(_ operation: Clear) async -> TimerId { await time.clear(operation) }
/// }
/// ```
///
/// ## What a timer answers with, and when
///
/// `notifyAt` and `notifyAfter` are answered exactly once, with the timer's own
/// id, when it fires. `clear` cancels the timer it names and is answered with
/// the same id.
///
/// A cleared timer's `notifyAt` or `notifyAfter` may still answer — a shell
/// need not race the two. By then the core has stopped listening for that
/// answer and ignores it, which is why the implementation below settles the
/// pending call rather than leaving it hanging for the process's lifetime.
public protocol TimeHandler: Sendable {
    /// The current wall-clock time.
    func now(_ operation: Now) async -> Instant
    /// Answer with `operation.id` once `operation.instant` has arrived.
    func notifyAt(_ operation: NotifyAt) async -> TimerId
    /// Answer with `operation.id` once `operation.duration` has elapsed.
    func notifyAfter(_ operation: NotifyAfter) async -> TimerId
    /// Cancel the timer `operation.id` names, and answer with it.
    func clear(_ operation: Clear) async -> TimerId
}

/// A `TimeHandler` whose timers are sleeping `Task`s.
///
/// The timer table is state, so the app constructs one handler where it
/// constructs its own. The table is guarded by a lock rather than by an actor,
/// because an actor would make every call a hop across an isolation boundary,
/// and the operations and outputs that would have to cross it are generated
/// types, which are not `Sendable`.
public final class TaskTimeHandler: TimeHandler, @unchecked Sendable {
    /// The timers that have not fired or been cleared, by id, and the lock
    /// that makes the table safe to reach from every operation at once.
    private var timers: [UInt64: Task<Void, Never>] = [:]
    private let lock = NSLock()

    public init() {}

    public func now(_: Now) async -> Instant {
        Self.instant(from: Date())
    }

    public func notifyAt(_ operation: NotifyAt) async -> TimerId {
        let target = Date(
            timeIntervalSince1970: Double(operation.instant.seconds)
                + Double(operation.instant.nanos) / 1_000_000_000
        )
        let seconds = target.timeIntervalSinceNow
        let nanoseconds = seconds <= 0 ? 0 : UInt64(seconds * 1_000_000_000)
        return await sleep(id: operation.id, nanoseconds: nanoseconds)
    }

    public func notifyAfter(_ operation: NotifyAfter) async -> TimerId {
        await sleep(id: operation.id, nanoseconds: operation.duration.nanos)
    }

    public func clear(_ operation: Clear) async -> TimerId {
        // Cancelling wakes the sleeping task, so the call waiting on it returns
        // and answers too. Nothing acts on that answer.
        forget(id: operation.id.value)?.cancel()
        return operation.id
    }

    private func sleep(id: TimerId, nanoseconds: UInt64) async -> TimerId {
        // The task is created and remembered under one hold of the lock, so a
        // `clear` cannot arrive in between and find nothing to cancel — which
        // would leave this timer sleeping out its whole duration. Creating a
        // task only schedules its body, so nothing runs while the lock is
        // held.
        let sleeping = remember(id: id.value) {
            Task<Void, Never> { try? await Task.sleep(nanoseconds: nanoseconds) }
        }
        // The lock is not held while the task sleeps, so `clear` can run.
        await sleeping.value
        _ = forget(id: id.value)
        return id
    }

    private func remember(id: UInt64, make: () -> Task<Void, Never>) -> Task<Void, Never> {
        lock.lock()
        defer { lock.unlock() }
        let timer = make()
        timers[id] = timer
        return timer
    }

    private func forget(id: UInt64) -> Task<Void, Never>? {
        lock.lock()
        defer { lock.unlock() }
        return timers.removeValue(forKey: id)
    }

    /// `Date` counts seconds from the same epoch `Instant` does, so this is
    /// only a matter of splitting the fraction off.
    private static func instant(from date: Date) -> Instant {
        let interval = date.timeIntervalSince1970
        let seconds = interval < 0 ? 0 : UInt64(interval)
        let nanos = UInt32(min((interval - Double(seconds)) * 1_000_000_000, 999_999_999))
        return Instant(seconds: seconds, nanos: nanos)
    }
}
