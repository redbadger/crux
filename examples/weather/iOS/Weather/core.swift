import App
import CoreLocation
import Foundation
import Shared
import UIKit
import os.log

// ANCHOR: core_base
@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel
    private let logger = Logger(subsystem: "com.example.weather", category: "Core")
    private let keyValueStore: KeyValueStore
    private var isInitialized = false
    private var core: CoreFfi

    init() {
        logger.info("Initializing Core")
        self.core = CoreFfi()

        self.view = try! .bincodeDeserialize(input: [UInt8](core.view()))

        do {
            self.keyValueStore = try KeyValueStore()
            logger.debug("KeyValueStore initialized successfully")
        } catch {
            logger.error("Failed to initialize KeyValueStore: \(error.localizedDescription)")
            fatalError("KeyValueStore initialization failed: \(error)")
        }
    }

    func update(_ event: Event) {
        let effects = [UInt8](core.update(Data(try! event.bincodeSerialize())))

        let requests: [Request] = try! .bincodeDeserialize(input: effects)
        for request in requests {
            processEffect(request)
        }
    }

    func processEffect(_ request: Request) {
        // ANCHOR_END: core_base
        switch request.effect {
        case .render:
            DispatchQueue.main.async {
                self.view = try! .bincodeDeserialize(input: [UInt8](self.core.view()))
            }
        // ANCHOR: http
        case .http(let req):
            logger.info("Making HTTP request to: \(req.url)")
            Task {
                do {
                    let response = try await requestHttp(req).get()
                    logger.debug("Received HTTP response with status: \(response.status)")

                    let effects = [UInt8](
                        core.resolve(
                            request.id,
                            Data(try! HttpResult.ok(response).bincodeSerialize())
                        )
                    )

                    let requests: [Request] = try! .bincodeDeserialize(input: effects)
                    logger.debug("Received \(requests.count) effects from HTTP response")

                    for request in requests {
                        logger.debug(
                            "Processing effect from HTTP response: \(String(describing: request.effect))"
                        )
                        processEffect(request)
                    }
                } catch {
                    logger.error("HTTP request failed: \(error.localizedDescription)")
                }
            }
        // ANCHOR_END: http
        case .keyValue(let keyValue):
            logger.debug("Processing KeyValue effect: \(String(describing: keyValue))")
            Task {
                do {
                    let result: KeyValueResult

                    switch keyValue {
                    case .get(let key):
                        logger.debug("Getting value for key: \(key)")
                        let value = keyValueStore.get(key: key)
                        logger.debug("Retrieved value: \(value.isEmpty ? "empty" : value)")
                        if !value.isEmpty {
                            // Convert string to Value type
                            let valueData = value.data(using: .utf8) ?? Data()
                            result = .ok(response: .get(value: .bytes([UInt8](valueData))))
                        } else {
                            result = .ok(response: .get(value: .bytes([])))
                        }

                    case .set(let key, let value):
                        logger.debug("Setting value for key: \(key)")
                        // Convert [UInt8] to String for storage
                        let valueString = String(bytes: value, encoding: .utf8) ?? ""
                        logger.debug("Value to store: \(valueString)")
                        // Get previous value if any
                        let previousValue = keyValueStore.get(key: key)
                        let previousData = previousValue.data(using: .utf8) ?? Data()
                        keyValueStore.set(key: key, value: valueString)
                        logger.debug("Value stored successfully")
                        result = .ok(response: .set(previous: .bytes([UInt8](previousData))))

                    case .delete(let key):
                        logger.debug("Deleting key: \(key)")
                        // Get previous value if any
                        let previousValue = keyValueStore.get(key: key)
                        let previousData = previousValue.data(using: .utf8) ?? Data()
                        keyValueStore.delete(key: key)
                        logger.debug("Key deleted successfully")
                        result = .ok(response: .delete(previous: .bytes([UInt8](previousData))))

                    case .exists(let key):
                        logger.debug("Checking existence of key: \(key)")
                        let exists = keyValueStore.exists(key: key)
                        logger.debug("Key exists: \(exists)")
                        result = .ok(response: .exists(is_present: exists))

                    case .listKeys(let prefix, let cursor):
                        logger.debug(
                            "Listing keys with prefix: \(prefix), cursor: \(String(describing: cursor))"
                        )
                        let keys = keyValueStore.listKeys(prefix: prefix, cursor: String(cursor))
                        logger.debug("Found keys: \(keys)")
                        // For simplicity, we'll use 0 as next_cursor since we don't implement pagination
                        result = .ok(response: .listKeys(keys: keys, next_cursor: 0))
                    }

                    let effects = [UInt8](
                        core.resolve(
                            request.id,
                            Data(try! result.bincodeSerialize())
                        )
                    )

                    let requests: [Request] = try! .bincodeDeserialize(input: effects)
                    logger.debug("Received \(requests.count) effects from KeyValue response")

                    for request in requests {
                        logger.debug(
                            "Processing effect from KeyValue response: \(String(describing: request.effect))"
                        )
                        processEffect(request)
                    }
                }
            }

        case .location(let locationOp):
            logger.debug("Processing Location effect: \(String(describing: locationOp))")

            Task {
                do {
                    let result: LocationResult
                    switch locationOp {
                    case .isLocationEnabled:
                        let enabled = await Task.detached {
                            CLLocationManager.locationServicesEnabled()
                        }.value
                        result = .enabled(enabled)

                    case .getLocation:
                        do {
                            let location = try await getCurrentLocationSafely()
                            let locationResponse = Location(
                                lat: location.coordinate.latitude,
                                lon: location.coordinate.longitude)
                            result = .location(locationResponse)
                        } catch {
                            logger.error("Failed to fetch location: \(error.localizedDescription)")
                            result = .location(nil)
                        }
                    }

                    let effects = [UInt8](
                        core.resolve(
                            request.id,
                            Data(try result.bincodeSerialize())
                        )
                    )
                    let requests: [Request] = try .bincodeDeserialize(input: effects)
                    logger.debug("Received \(requests.count) effects from Location response")
                    for request in requests {
                        logger.debug(
                            "Processing effect from Location response: \(String(describing: request.effect))"
                        )
                        processEffect(request)
                    }
                } catch {
                    logger.error("Failed to process location effect: \(error.localizedDescription)")
                    // Even if something goes wrong with the effect processing, return nil location
                    let result = LocationResult.location(nil)
                    let effects = [UInt8](
                        core.resolve(
                            request.id,
                            Data(try result.bincodeSerialize())
                        )
                    )
                    let requests: [Request] = try .bincodeDeserialize(input: effects)
                    for request in requests {
                        processEffect(request)
                    }
                }
            }

        }
    }
}

extension Core {
    private func getCurrentLocationSafely() async throws -> CLLocation {
        return try await withCheckedThrowingContinuation {
            (continuation: CheckedContinuation<CLLocation, Error>) in
            class LocationDelegate: NSObject, CLLocationManagerDelegate {
                let continuation: CheckedContinuation<CLLocation, Error>
                var manager: CLLocationManager?
                private var hasResumed = false
                private var timeoutTask: Task<Void, Never>?
                private let resumeLock = NSLock()

                init(
                    manager: CLLocationManager, continuation: CheckedContinuation<CLLocation, Error>
                ) {
                    self.manager = manager
                    self.continuation = continuation
                    super.init()
                    self.manager?.delegate = self

                    // Set up timeout with proper coordination
                    self.timeoutTask = Task { [weak self] in
                        try? await Task.sleep(nanoseconds: 15_000_000_000)  // 15 seconds
                        self?.handleTimeout()
                    }
                }

                private func handleTimeout() {
                    safeResume {
                        continuation.resume(
                            throwing: NSError(
                                domain: "LocationError", code: -1,
                                userInfo: [NSLocalizedDescriptionKey: "Location request timed out"])
                        )
                    }
                }

                private func safeResume(_ action: () -> Void) {
                    resumeLock.lock()
                    defer { resumeLock.unlock() }

                    guard !hasResumed else { return }
                    hasResumed = true
                    timeoutTask?.cancel()
                    action()
                    cleanup()
                }

                private func cleanup() {
                    manager?.stopUpdatingLocation()
                    manager?.delegate = nil
                    manager = nil
                }

                func locationManager(
                    _ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]
                ) {
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
                                    userInfo: [
                                        NSLocalizedDescriptionKey: "Location access not available"
                                    ]))
                        }
                    case .authorizedWhenInUse, .authorizedAlways:
                        Task.detached { [weak self, weak manager] in
                            let enabled = CLLocationManager.locationServicesEnabled()
                            await MainActor.run {
                                guard let manager = manager, let self = self else { return }
                                if enabled {
                                    manager.startUpdatingLocation()
                                } else {
                                    self.safeResume {
                                        self.continuation.resume(
                                            throwing: NSError(
                                                domain: "LocationError", code: -1,
                                                userInfo: [
                                                    NSLocalizedDescriptionKey:
                                                        "Location services are disabled"
                                                ]))
                                    }
                                }
                            }
                        }
                    case .notDetermined:
                        // Wait for user decision
                        break
                    @unknown default:
                        safeResume {
                            continuation.resume(
                                throwing: NSError(
                                    domain: "LocationError", code: -1,
                                    userInfo: [
                                        NSLocalizedDescriptionKey: "Unknown authorization status"
                                    ]))
                        }
                    }
                }

                deinit {
                    cleanup()
                    timeoutTask?.cancel()
                }
            }

            let locationManager = CLLocationManager()
            let delegate = LocationDelegate(manager: locationManager, continuation: continuation)

            // Store the delegate in a strong reference to prevent deallocation
            objc_setAssociatedObject(
                locationManager, "delegate", delegate, .OBJC_ASSOCIATION_RETAIN)

            // Configure location manager
            locationManager.desiredAccuracy = kCLLocationAccuracyBest
            locationManager.distanceFilter = kCLDistanceFilterNone

            // Check authorization status asynchronously but off main thread to avoid warnings
            Task.detached {
                let isLocationEnabled = CLLocationManager.locationServicesEnabled()
                await MainActor.run {
                    let status = locationManager.authorizationStatus
                    switch status {
                    case .denied, .restricted:
                        delegate.locationManagerDidChangeAuthorization(locationManager)
                    case .notDetermined:
                        locationManager.requestWhenInUseAuthorization()
                    case .authorizedWhenInUse, .authorizedAlways:
                        if isLocationEnabled {
                            locationManager.startUpdatingLocation()
                        } else {
                            delegate.locationManagerDidChangeAuthorization(locationManager)
                        }
                    @unknown default:
                        delegate.locationManagerDidChangeAuthorization(locationManager)
                    }
                }
            }
        }
    }
}
