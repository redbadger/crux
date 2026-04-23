import App
import Foundation

private let logger = Log.time

extension Core {
    func resolveTime(request: TimeRequest, requestId: UInt32) {
        switch request {
        case .now:
            let now = Date()
            logger.debug("sending current time")
            let response = TimeResponse.now(
                instant: Instant(
                    seconds: UInt64(now.timeIntervalSince1970),
                    nanos: UInt32(
                        (now.timeIntervalSince1970.truncatingRemainder(dividingBy: 1))
                            * 1_000_000_000)
                )
            )
            resolve(requestId: requestId, serialize: { try response.bincodeSerialize() })

        case let .notifyAt(id: timerId, instant: instant):
            let targetDate = Date(timeIntervalSince1970: Double(instant.seconds))
            let interval = max(targetDate.timeIntervalSinceNow, 0)
            scheduleTimer(id: timerId, interval: interval, requestId: requestId) { id in
                TimeResponse.instantArrived(id: TimerId(value: id))
            }

        case let .notifyAfter(id: timerId, duration: duration):
            let interval = TimeInterval(duration.nanos) / 1_000_000_000
            scheduleTimer(id: timerId, interval: interval, requestId: requestId) { id in
                TimeResponse.durationElapsed(id: TimerId(value: id))
            }

        case let .clear(id: timerId):
            let id = timerId.value
            logger.debug("clearing timer (\(id))")
            activeTimers[id]?.invalidate()
            activeTimers.removeValue(forKey: id)
            let response = TimeResponse.cleared(id: timerId)
            resolve(requestId: requestId, serialize: { try response.bincodeSerialize() })
        }
    }

    private func scheduleTimer(
        id timerId: TimerId,
        interval: TimeInterval,
        requestId: UInt32,
        response: @Sendable @escaping (UInt64) -> TimeResponse
    ) {
        let id = timerId.value
        logger.debug("scheduling timer (\(id)) for \(interval)s")
        let timer = Timer.scheduledTimer(
            withTimeInterval: interval, repeats: false
        ) { _ in
            Task { @MainActor in
                logger.debug("timer (\(id)) elapsed")
                let resp = response(id)
                self.resolve(
                    requestId: requestId,
                    serialize: { try resp.bincodeSerialize() }
                )
                self.activeTimers.removeValue(forKey: id)
            }
        }
        activeTimers[id] = timer
    }
}
