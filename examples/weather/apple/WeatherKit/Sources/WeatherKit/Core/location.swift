@preconcurrency import CoreLocation

import App
import Foundation

private nonisolated let logger = Log.location

nonisolated extension Core {
    /// `IsLocationEnabled` is answered with a bare `Bool` — the operation's
    /// output type, so there is nothing to unwrap on either side.
    public func isLocationEnabled(_: IsLocationEnabled) async -> Bool {
        let enabled = CLLocationManager.locationServicesEnabled()
        logger.debug("location enabled: \(enabled)")
        return enabled
    }

    /// `GetLocation` is answered with `Location?`: the coordinates, or nothing
    /// if the shell could not get a fix.
    public func getLocation(_: GetLocation) async -> Location? {
        guard let coordinate = await currentCoordinate() else {
            return nil
        }
        logger.debug("location: \(coordinate.latitude), \(coordinate.longitude)")
        return Location(lat: coordinate.latitude, lon: coordinate.longitude)
    }

    /// CoreLocation's delegate plumbing runs on the main actor; only the
    /// coordinates come back.
    @MainActor
    private func currentCoordinate() async -> (latitude: Double, longitude: Double)? {
        do {
            let location = try await getCurrentLocation()
            return (location.coordinate.latitude, location.coordinate.longitude)
        } catch {
            logger.warning("location failed: \(error.localizedDescription)")
            return nil
        }
    }

    @MainActor
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
