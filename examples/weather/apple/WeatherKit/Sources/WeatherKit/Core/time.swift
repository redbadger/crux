import App
import Foundation

private nonisolated let logger = Log.time

nonisolated extension Core {
    /// `TimeNotifyAfter` is a request: it is answered exactly once, with the
    /// id of the timer that fired.
    ///
    /// If `timeClear` arrives first this method never returns. `Clear` is a
    /// notification, so by the time the shell sees it the core has already
    /// stopped waiting for the timer — answering it then would be answering a
    /// question nobody is asking.
    public func timeNotifyAfter(_ operation: NotifyAfter) async -> TimerId {
        let id = operation.id.value
        let interval = TimeInterval(operation.duration.nanos) / 1_000_000_000

        await withUnsafeContinuation { continuation in
            Task { @MainActor in
                scheduleTimer(id: id, interval: interval, elapsed: continuation)
            }
        }

        return operation.id
    }

    /// `TimeClear` is a notification: release the timer and answer nothing.
    public func timeClear(_ operation: Clear) {
        let id = operation.id.value
        Task { @MainActor in cancelTimer(id: id) }
    }

    /// `Timer` needs a run loop, so the timer table lives on the main actor.
    @MainActor
    private func scheduleTimer(
        id: UInt64,
        interval: TimeInterval,
        elapsed: UnsafeContinuation<Void, Never>
    ) {
        logger.debug("scheduling timer (\(id)) for \(interval)s")
        activeTimers[id] = Timer.scheduledTimer(
            withTimeInterval: interval, repeats: false
        ) { _ in
            Task { @MainActor in
                logger.debug("timer (\(id)) elapsed")
                self.activeTimers.removeValue(forKey: id)
                elapsed.resume()
            }
        }
    }

    /// Invalidating the timer releases its closure, and with it the
    /// continuation `timeNotifyAfter` is suspended on — which is exactly what
    /// we want, because nothing is expecting an answer any more.
    @MainActor
    private func cancelTimer(id: UInt64) {
        logger.debug("clearing timer (\(id))")
        activeTimers.removeValue(forKey: id)?.invalidate()
    }
}
