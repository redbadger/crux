import Foundation
import SharedTypes
import UIKit
import os.log

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel
    private let logger = Logger(subsystem: "com.example.weather", category: "Core")
    private let keyValueStore: KeyValueStore

    init() {
        logger.info("Initializing Core")
        self.view = try! .bincodeDeserialize(input: [UInt8](Weather.view()))
        self.keyValueStore = KeyValueStore()
        logger.debug("Initial view state: \(String(describing: self.view))")
        
        // Restore favorites when app starts
        logger.info("Triggering favorites restore on app start")
        update(.favorites(.restore))
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
                        // TODO: Implement actual location permission check
                        let enabled = true // Replace with real check
                        result = .enabled(enabled)
                    case .getLocation:
                        // TODO: Implement actual location fetching
                        let lat = 37.7749
                        let lon = -122.4194
                        let location = SharedTypes.LocationResponse(lat: lat, lon: lon)
                        result = .location(location)
                    }
                    let effects = [UInt8](
                        handleResponse(
                            request.id,
                            Data(try! result.bincodeSerialize())
                        )
                    )
                    let requests: [Request] = try! .bincodeDeserialize(input: effects)
                    logger.debug("Received \(requests.count) effects from Location response")
                    for request in requests {
                        logger.debug("Processing effect from Location response: \(String(describing: request.effect))")
                        processEffect(request)
                    }
                }
            }
        }
        
        
    }
}
