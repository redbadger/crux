import Foundation

/// A `TimeHandler` whose timers are sleeping `Task`s.
///
/// The app's handler holds an instance and delegates each operation to it:
///
/// ```swift
/// struct MyHandler: EffectHandler {
///     let time = TaskTimeHandler()
///
///     func timeNotifyAfter(_ operation: NotifyAfter) async -> TimerId { await time.notifyAfter(operation) }
///     func timeClear(_ operation: ClearTimer) async -> TimerId { await time.clear(operation) }
/// }
/// ```
///
/// The timer table is state, so the app constructs one handler where it
/// constructs its own. The table is guarded by a lock rather than by an actor,
/// because an actor would make every call a hop across an isolation boundary,
/// and the operations and outputs that would have to cross it are generated
/// types, which are not `Sendable`.
///
/// ## Precision
///
/// `now` reads `Date`, which keeps seconds as a `Double`. Near the present that
/// resolves about 240 ns, finer than a microsecond. `Task.sleep` takes
/// nanoseconds. `notifyAfter` passes its duration to `Task.sleep` unchanged,
/// so it does not fire early. `notifyAt` builds its deadline as a `Double`,
/// and the conversion back to nanoseconds truncates. It can fire early by
/// under a microsecond. Both timers can fire late, because the system
/// schedules the task.
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
        let nanoseconds = Self.nanoseconds(target.timeIntervalSinceNow)
        return await sleep(id: operation.id, nanoseconds: nanoseconds)
    }

    public func notifyAfter(_ operation: NotifyAfter) async -> TimerId {
        await sleep(id: operation.id, nanoseconds: operation.duration.nanos)
    }

    public func clear(_ operation: ClearTimer) async -> TimerId {
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

    /// `seconds` as whole nanoseconds, from 0 to `UInt64.max`.
    ///
    /// The conversion to `UInt64` traps for a value it cannot hold, which an
    /// instant more than about 584 years ahead gives. `Double(UInt64.max)` is
    /// 2^64 exactly, so a product below it converts.
    private static func nanoseconds(_ seconds: TimeInterval) -> UInt64 {
        let nanoseconds = seconds * 1_000_000_000
        if nanoseconds <= 0 { return 0 }
        if nanoseconds >= Double(UInt64.max) { return UInt64.max }
        return UInt64(nanoseconds)
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
