@preconcurrency import CoreLocation

import App
import Foundation

private let logger = Log.location

extension Core {
    func resolveLocation(request: LocationOperation, requestId: UInt32) {
        Task {
            let result: LocationResult
            switch request {
            case .isLocationEnabled:
                let enabled = CLLocationManager.locationServicesEnabled()
                logger.debug("location enabled: \(enabled)")
                result = .enabled(enabled)

            case .getLocation:
                do {
                    let location = try await getCurrentLocation()
                    logger.debug("location: \(location.coordinate.latitude), \(location.coordinate.longitude)")
                    result = .location(
                        Location(
                            lat: location.coordinate.latitude,
                            lon: location.coordinate.longitude
                        )
                    )
                } catch {
                    logger.warning("location failed: \(error.localizedDescription)")
                    result = .location(nil)
                }
            }

            resolve(requestId: requestId, serialize: { try result.bincodeSerialize() })
        }
    }

    private func getCurrentLocation() async throws -> CLLocation {
        try await withCheckedThrowingContinuation { continuation in
            let manager = CLLocationManager()
            let delegate = LocationDelegate(manager: manager, continuation: continuation)

            objc_setAssociatedObject(manager, "delegate", delegate, .OBJC_ASSOCIATION_RETAIN)

            manager.desiredAccuracy = kCLLocationAccuracyBest
            manager.distanceFilter = kCLDistanceFilterNone

            let status = manager.authorizationStatus
            switch status {
            case .denied, .restricted:
                delegate.locationManagerDidChangeAuthorization(manager)
            case .notDetermined:
                manager.requestWhenInUseAuthorization()
            case .authorizedWhenInUse, .authorizedAlways:
                if CLLocationManager.locationServicesEnabled() {
                    manager.startUpdatingLocation()
                } else {
                    delegate.locationManagerDidChangeAuthorization(manager)
                }
            @unknown default:
                delegate.locationManagerDidChangeAuthorization(manager)
            }
        }
    }
}

private class LocationDelegate: NSObject, CLLocationManagerDelegate {
    let continuation: CheckedContinuation<CLLocation, Error>
    var manager: CLLocationManager?
    private var hasResumed = false
    private var timeoutTask: Task<Void, Never>?
    private let resumeLock = NSLock()

    init(manager: CLLocationManager, continuation: CheckedContinuation<CLLocation, Error>) {
        self.manager = manager
        self.continuation = continuation
        super.init()
        self.manager?.delegate = self

        self.timeoutTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: 15_000_000_000)
            self?.handleTimeout()
        }
    }

    private func handleTimeout() {
        safeResume {
            continuation.resume(
                throwing: NSError(
                    domain: "LocationError", code: -1,
                    userInfo: [NSLocalizedDescriptionKey: "Location request timed out"]))
        }
    }

    private func safeResume(_ action: () -> Void) {
        resumeLock.lock()
        defer { resumeLock.unlock() }

        guard !hasResumed else { return }
        hasResumed = true
        timeoutTask?.cancel()
        action()
        manager?.stopUpdatingLocation()
        manager?.delegate = nil
        manager = nil
    }

    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        if let location = locations.first {
            safeResume {
                continuation.resume(returning: location)
            }
        }
    }

    func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        safeResume {
            continuation.resume(throwing: error)
        }
    }

    func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        switch manager.authorizationStatus {
        case .denied, .restricted:
            safeResume {
                continuation.resume(
                    throwing: NSError(
                        domain: "LocationError", code: -1,
                        userInfo: [NSLocalizedDescriptionKey: "Location access not available"]))
            }
        case .authorizedWhenInUse, .authorizedAlways:
            if CLLocationManager.locationServicesEnabled() {
                manager.startUpdatingLocation()
            } else {
                safeResume {
                    continuation.resume(
                        throwing: NSError(
                            domain: "LocationError", code: -1,
                            userInfo: [NSLocalizedDescriptionKey: "Location services are disabled"]))
                }
            }
        case .notDetermined:
            break
        @unknown default:
            safeResume {
                continuation.resume(
                    throwing: NSError(
                        domain: "LocationError", code: -1,
                        userInfo: [NSLocalizedDescriptionKey: "Unknown authorization status"]))
            }
        }
    }
}
