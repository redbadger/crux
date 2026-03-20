import App
import SwiftUI

enum SseError: Error {
    case generic(Error)
    case message(String)
}

func requestSse(_ request: SseRequest) async -> AsyncStream<Result<SseResponse, SseError>> {
    return AsyncStream { continuation in
        Task {
            let req = URLRequest(url: URL(string: request.url)!)
            do {
                let (asyncBytes, response) = try await URLSession.shared.bytes(for: req)
                if let httpResponse = response as? HTTPURLResponse {
                    if !(200 ... 299).contains(httpResponse.statusCode) {
                        continuation.yield(.failure(
                            .message("error, status code: \(httpResponse.statusCode)")
                        ))
                        continuation.finish()
                        return
                    }
                }

                for try await line in asyncBytes.lines {
                    let line = line + "\n\n"
                    continuation.yield(.success(.chunk([UInt8](line.utf8))))
                }
                continuation.yield(.success(.done))
                continuation.finish()
            } catch {
                continuation.yield(.failure(.generic(error)))
                continuation.finish()
            }
        }
    }
}
