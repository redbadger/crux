import Foundation
import SharedTypes
import UIKit
import os.log

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel
    private let logger = Logger(subsystem: "com.example.weather", category: "Core")

    init() {
        logger.info("Initializing Core")
        self.view = try! .bincodeDeserialize(input: [UInt8](Weather.view()))
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
            
        case .keyValue:
            logger.debug("Processing KeyValue effect")
        }
    }
}
