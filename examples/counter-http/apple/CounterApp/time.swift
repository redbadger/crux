import App
import Foundation

/// Runs the timers of `crux_time` as sleeping tasks.
///
/// The handler keeps a table of the timers that run, so that a `ClearTimer`
/// can cancel the timer that it names.
final class TaskTimeHandler: Sendable {
    private let timers = TimerTable()

    /// `NotifyAfter` is answered exactly once, with the id of the timer that
    /// fired.
    ///
    /// If a `ClearTimer` for the timer arrives first, it cancels the sleeping
    /// task, and this method answers early. The core ignores that answer,
    /// because it stops listening for this request when it clears the timer.
    func notifyAfter(_ operation: NotifyAfter) async -> TimerId {
        await timers.sleep(id: operation.id.value, nanoseconds: UInt64(operation.duration.nanos))

        return operation.id
    }

    /// `ClearTimer` cancels the timer and answers with the id that it names.
    func clear(_ operation: ClearTimer) async -> TimerId {
        await timers.cancel(id: operation.id.value)

        return operation.id
    }
}

/// The table of the timers that run. The table is an actor, so only one call
/// at a time changes it, and only `Sendable` values (ids and durations) go
/// into it.
private actor TimerTable {
    private var activeTimers: [UInt64: Task<Void, Never>] = [:]

    func sleep(id: UInt64, nanoseconds: UInt64) async {
        let sleeping = Task<Void, Never> { try? await Task.sleep(nanoseconds: nanoseconds) }
        activeTimers[id] = sleeping
        await sleeping.value
        activeTimers.removeValue(forKey: id)
    }

    func cancel(id: UInt64) {
        activeTimers.removeValue(forKey: id)?.cancel()
    }
}
