import App
import Foundation
import Synchronization

private nonisolated let logger = Log.time

/// The app's `TimeHandler`, over `Task.sleep`.
///
/// The handler answers each timer exactly once, with the id of the timer. If
/// `clear` arrives first, it cancels the sleeping task, and the pending call
/// then answers immediately. The core ignores that answer, because it no
/// longer listens for it.
nonisolated final class TaskSleepTimeHandler: TimeHandler {
    /// The sleeping tasks, so that `clear` can cancel the task that it names.
    private let activeTimers = Mutex<[UInt64: Task<Void, Never>]>([:])

    func now(_ operation: Now) async -> Instant {
        let since = Date().timeIntervalSince1970
        let seconds = since.rounded(.down)
        return Instant(seconds: UInt64(seconds), nanos: UInt32((since - seconds) * 1_000_000_000))
    }

    func notifyAt(_ operation: NotifyAt) async -> TimerId {
        let instant = operation.instant
        let target = Double(instant.seconds) + Double(instant.nanos) / 1_000_000_000
        let delay = max(0, target - Date().timeIntervalSince1970)
        return await sleep(nanoseconds: UInt64(delay * 1_000_000_000), id: operation.id)
    }

    func notifyAfter(_ operation: NotifyAfter) async -> TimerId {
        await sleep(nanoseconds: UInt64(operation.duration.nanos), id: operation.id)
    }

    func clear(_ operation: ClearTimer) async -> TimerId {
        let id = operation.id.value
        logger.debug("clearing timer (\(id))")
        let sleeping = activeTimers.withLock { $0.removeValue(forKey: id) }
        sleeping?.cancel()
        return operation.id
    }

    private func sleep(nanoseconds: UInt64, id: TimerId) async -> TimerId {
        let key = id.value
        logger.debug("scheduling timer (\(key)) for \(nanoseconds)ns")
        let sleeping = Task<Void, Never> { try? await Task.sleep(nanoseconds: nanoseconds) }
        activeTimers.withLock { $0[key] = sleeping }
        await sleeping.value
        activeTimers.withLock { timers in
            if timers[key] == sleeping {
                timers.removeValue(forKey: key)
            }
        }
        logger.debug("timer (\(key)) finished")
        return id
    }
}
