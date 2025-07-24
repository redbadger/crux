import Foundation
import SharedTypes
import UIKit
import os.log
import CoreLocation

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel
    private let logger = Logger(subsystem: "com.example.weather", category: "Core")
    private let keyValueStore: KeyValueStore
    private var isInitialized = false
    
    init() {
        logger.info("Initializing Core")
        
        // Initialize with default view first to prevent crashes
        do {
            self.view = try .bincodeDeserialize(input: [UInt8](Weather.view()))
        } catch {
            logger.error("Failed to initialize view: \(error.localizedDescription)")
            // Create a minimal default view as fallback
            let defaultWeatherData = CurrentResponse(
                coord: Coord(lat: 0.0, lon: 0.0),
                weather: [],
                base: "",
                main: Main(temp: 0.0, feels_like: 0.0, temp_min: 0.0, temp_max: 0.0, pressure: 0, humidity: 0),
                visibility: 0,
                wind: Wind(speed: 0.0, deg: 0, gust: nil),
                clouds: Clouds(all: 0),
                dt: 0,
                sys: Sys(type: 0, id: 0, country: "", sunrise: 0, sunset: 0),
                timezone: 0,
                id: 0,
                name: "",
                cod: 0
            )
            self.view = ViewModel(workflow: .home(weather_data: defaultWeatherData, favorites: []))
        }
        
        do {
            self.keyValueStore = try KeyValueStore()
            logger.debug("KeyValueStore initialized successfully")
        } catch {
            logger.error("Failed to initialize KeyValueStore: \(error.localizedDescription)")
            fatalError("KeyValueStore initialization failed: \(error)")
        }
        
        logger.debug("Initial view state: \(String(describing: self.view))")
    }
    
    func update(_ event: Event) {
        logger.info("Processing event: \(String(describing: event))")
        let effects = [UInt8](processEvent(Data(try! event.bincodeSerialize())))
        
        let requests: [Request] = try! .bincodeDeserialize(input: effects)
        logger.debug("Received \(requests.count) effects to process")
        
        for request in requests {
            logger.debug("Processing effect: \(String(describing: request.effect))")
            processEffect(request)
        }
    }
    
    func processEffect(_ request: Request) {
        switch request.effect {
        case .render:
            logger.info("Rendering new view state")
            self.view = try! .bincodeDeserialize(input: [UInt8](Weather.view()))
            
        case let .http(req):
            logger.info("Making HTTP request to: \(req.url)")
            Task {
                do {
                    let response = try await requestHttp(req).get()
                    logger.debug("Received HTTP response with status: \(response.status)")
                    
                    let effects = [UInt8](
                        handleResponse(
                            request.id,
                            Data(try! HttpResult.ok(response).bincodeSerialize())
                        )
                    )
                    
                    let requests: [Request] = try! .bincodeDeserialize(input: effects)
                    logger.debug("Received \(requests.count) effects from HTTP response")
                    
                    for request in requests {
                        logger.debug("Processing effect from HTTP response: \(String(describing: request.effect))")
                        processEffect(request)
                    }
                } catch {
                    logger.error("HTTP request failed: \(error.localizedDescription)")
                }
            }
            
        case let .keyValue(keyValue):
            
            logger.debug("Processing KeyValue effect: \(String(describing: keyValue))")
            Task {
                do {
                    let result: SharedTypes.KeyValueResult
                    
                    switch keyValue {
                    case .get(key: let key):
                        logger.debug("Getting value for key: \(key)")
                        let value = keyValueStore.get(key: key)
                        logger.debug("Retrieved value: \(value.isEmpty ? "empty" : value)")
                        if !value.isEmpty {
                            // Convert string to Value type
                            let valueData = value.data(using: .utf8) ?? Data()
                            result = .ok(response: .get(value: SharedTypes.Value.bytes([UInt8](valueData))))
                        } else {
                            result = .ok(response: .get(value: SharedTypes.Value.bytes([])))
                        }
                        
                    case .set(key: let key, value: let value):
                        logger.debug("Setting value for key: \(key)")
                        // Convert [UInt8] to String for storage
                        let valueString = String(bytes: value, encoding: .utf8) ?? ""
                        logger.debug("Value to store: \(valueString)")
                        // Get previous value if any
                        let previousValue = keyValueStore.get(key: key)
                        let previousData = previousValue.data(using: .utf8) ?? Data()
                        keyValueStore.set(key: key, value: valueString)
                        logger.debug("Value stored successfully")
                        result = .ok(response: .set(previous: SharedTypes.Value.bytes([UInt8](previousData))))
                        
                    case .delete(key: let key):
                        logger.debug("Deleting key: \(key)")
                        // Get previous value if any
                        let previousValue = keyValueStore.get(key: key)
                        let previousData = previousValue.data(using: .utf8) ?? Data()
                        keyValueStore.delete(key: key)
                        logger.debug("Key deleted successfully")
                        result = .ok(response: .delete(previous: SharedTypes.Value.bytes([UInt8](previousData))))
                        
                    case .exists(key: let key):
                        logger.debug("Checking existence of key: \(key)")
                        let exists = keyValueStore.exists(key: key)
                        logger.debug("Key exists: \(exists)")
                        result = .ok(response: .exists(is_present: exists))
                        
                    case .listKeys(prefix: let prefix, cursor: let cursor):
                        logger.debug("Listing keys with prefix: \(prefix), cursor: \(String(describing: cursor))")
                        let keys = keyValueStore.listKeys(prefix: prefix, cursor: String(cursor))
                        logger.debug("Found keys: \(keys)")
                        // For simplicity, we'll use 0 as next_cursor since we don't implement pagination
                        result = .ok(response: .listKeys(keys: keys, next_cursor: 0))
                    }
                    
                    let effects = [UInt8](
                        handleResponse(
                            request.id,
                            Data(try! result.bincodeSerialize())
                        )
                    )
                    
                    let requests: [Request] = try! .bincodeDeserialize(input: effects)
                    logger.debug("Received \(requests.count) effects from KeyValue response")
                    
                    for request in requests {
                        logger.debug("Processing effect from KeyValue response: \(String(describing: request.effect))")
                        processEffect(request)
                    }
                }
            }
            
        case let .location(locationOp):
            logger.debug("Processing Location effect: \(String(describing: locationOp))")
            
            Task {
                do {
                    let result: SharedTypes.LocationResult
                    switch locationOp {
                    case .isLocationEnabled:
                        let enabled = CLLocationManager.locationServicesEnabled()
                        result = .enabled(enabled)
                        
                    case .getLocation:
                        do {
                            let location = try await getCurrentLocationSafely()
                            let locationResponse = SharedTypes.Location(lat: location.coordinate.latitude, lon: location.coordinate.longitude)
                            result = .location(locationResponse)
                        } catch {
                            logger.error("Failed to fetch location: \(error.localizedDescription)")
                            result = .location(nil)
                        }
                    }
                    
                    let effects = [UInt8](
                        handleResponse(
                            request.id,
                            Data(try result.bincodeSerialize())
                        )
                    )
                    let requests: [Request] = try .bincodeDeserialize(input: effects)
                    logger.debug("Received \(requests.count) effects from Location response")
                    for request in requests {
                        logger.debug("Processing effect from Location response: \(String(describing: request.effect))")
                        processEffect(request)
                    }
                } catch {
                    logger.error("Failed to process location effect: \(error.localizedDescription)")
                    // Even if something goes wrong with the effect processing, return nil location
                    let result = SharedTypes.LocationResult.location(nil)
                    let effects = [UInt8](
                        handleResponse(
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
        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<CLLocation, Error>) in
            class LocationDelegate: NSObject, CLLocationManagerDelegate {
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
                    
                    // Set up timeout with proper coordination
                    self.timeoutTask = Task { [weak self] in
                        try? await Task.sleep(nanoseconds: 15_000_000_000) // 15 seconds
                        self?.handleTimeout()
                    }
                }
                
                private func handleTimeout() {
                    safeResume {
                        continuation.resume(throwing: NSError(domain: "LocationError", code: -1, userInfo: [NSLocalizedDescriptionKey: "Location request timed out"]))
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
                            continuation.resume(throwing: NSError(domain: "LocationError", code: -1, userInfo: [NSLocalizedDescriptionKey: "Location access not available"]))
                        }
                    case .authorizedWhenInUse, .authorizedAlways:
                        if CLLocationManager.locationServicesEnabled() {
                            manager.startUpdatingLocation()
                        } else {
                            safeResume {
                                continuation.resume(throwing: NSError(domain: "LocationError", code: -1, userInfo: [NSLocalizedDescriptionKey: "Location services are disabled"]))
                            }
                        }
                    case .notDetermined:
                        // Wait for user decision
                        break
                    @unknown default:
                        safeResume {
                            continuation.resume(throwing: NSError(domain: "LocationError", code: -1, userInfo: [NSLocalizedDescriptionKey: "Unknown authorization status"]))
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
            objc_setAssociatedObject(locationManager, "delegate", delegate, .OBJC_ASSOCIATION_RETAIN)
            
            // Configure location manager
            locationManager.desiredAccuracy = kCLLocationAccuracyBest
            locationManager.distanceFilter = kCLDistanceFilterNone
            
            // Check authorization status asynchronously but off main thread to avoid warnings
            Task.detached {
                await MainActor.run {
                    let status = locationManager.authorizationStatus
                    switch status {
                    case .denied, .restricted:
                        delegate.locationManagerDidChangeAuthorization(locationManager)
                    case .notDetermined:
                        locationManager.requestWhenInUseAuthorization()
                    case .authorizedWhenInUse, .authorizedAlways:
                        if CLLocationManager.locationServicesEnabled() {
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
