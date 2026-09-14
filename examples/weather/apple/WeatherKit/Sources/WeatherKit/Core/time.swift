import App
import Foundation

private nonisolated let logger = Log.time

nonisolated extension Core {
    /// `TimeNotifyAfter` is a request: it is answered exactly once, with the
    /// id of the timer that fired.
    ///
    /// If `timeClear` arrives first the sleeping task is cancelled, and this
    /// method returns early and answers anyway. That is harmless: the core
    /// stopped listening for this request the moment it cleared the timer, and
    /// ignores the answer.
    public func timeNotifyAfter(_ operation: NotifyAfter) async -> TimerId {
        let id = operation.id.value
        let nanoseconds = UInt64(operation.duration.nanos)
        logger.debug("scheduling timer (\(id)) for \(nanoseconds)ns")

        let sleeping = Task<Void, Never> { try? await Task.sleep(nanoseconds: nanoseconds) }
        await store(timer: sleeping, id: id)
        await sleeping.value
        await forget(id: id)

        logger.debug("timer (\(id)) finished")
        return operation.id
    }

    /// `TimeClear` is a request: cancel the timer, release what it was holding,
    /// and answer with the id it named.
    public func timeClear(_ operation: Clear) async -> TimerId {
        let id = operation.id.value
        logger.debug("clearing timer (\(id))")
        await cancelTimer(id: id)

        return operation.id
    }

    /// The timer table lives on the main actor, so the handler methods hop to
    /// reach it.
    @MainActor
    private func store(timer: Task<Void, Never>, id: UInt64) {
        activeTimers[id] = timer
    }

    @MainActor
    private func forget(id: UInt64) {
        activeTimers.removeValue(forKey: id)
    }

    /// Cancelling the sleeping task wakes `timeNotifyAfter`, which then returns
    /// and answers its own request. Nothing acts on that answer — the core is
    /// no longer listening for it.
    @MainActor
    private func cancelTimer(id: UInt64) {
        activeTimers.removeValue(forKey: id)?.cancel()
    }
}
