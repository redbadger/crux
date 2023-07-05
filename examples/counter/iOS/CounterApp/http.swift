import SharedTypes
import SwiftUI

enum HttpError : Error {
    case generic(Error)
    case message(String)
}

func httpRequest(_ request: HttpRequest) async -> Result<HttpResponse, HttpError> {
    var req = URLRequest(url: URL(string: request.url)!)
    req.httpMethod = request.method
    
    for header in request.headers {
        req.addValue(header.value, forHTTPHeaderField: header.name)
    }
    
    do {
        let (data, response) = try await URLSession.shared.data(for: req)
        if let httpResponse = response as? HTTPURLResponse {
            let status = UInt16(httpResponse.statusCode)
            let body = [UInt8](data)
            return .success(HttpResponse(status: status, headers: [], body: body))
        } else {
            return .failure(.message("bad response"))
        }
    }
    catch {
        return .failure(.generic(error))
    }
}
