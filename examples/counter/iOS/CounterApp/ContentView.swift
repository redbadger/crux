import SharedTypes
import SwiftUI

enum Outcome {
    case http(HttpResponse)
    case sse(SseResponse)
}

typealias Uuid = [UInt8]

enum Message {
    case event(Event)
    case response(Uuid, Outcome)
}

@MainActor
class Model: ObservableObject {
    @Published var view = ViewModel(text: "", confirmed: false)

    init() {
        update(msg: .event(.startWatch))
    }
    
    func update(msg: Message) {
        var reqs: [Request]

        switch msg {
        case let .event(m):
            reqs = try! [Request].bincodeDeserialize(
                input: [UInt8](processEvent(Data(try! m.bincodeSerialize())))
            )
        case let .response(uuid, .http(r)):
            reqs = try! [Request].bincodeDeserialize(
                input: [UInt8](handleResponse(Data(uuid), Data(try! r.bincodeSerialize())))
            )
        case let .response(uuid, .sse(r)):
            reqs = try! [Request].bincodeDeserialize(
                input: [UInt8](handleResponse(Data(uuid), Data(try! r.bincodeSerialize())))
            )
        }

        for req in reqs {
            switch req.effect {
            case .render:
                view = try! ViewModel.bincodeDeserialize(
                    input: [UInt8](CounterApp.view())
                )
            case let .http(httpReq):
                Task {
                    let res = try! await http(request: httpReq).get()
                    update(msg: .response(req.uuid, .http(res)))
                }
            case let .serverSentEvents(sseReq):
                Task {
                    for await result in await sse(request: sseReq) {
                        update(msg: .response(req.uuid, .sse(try! result.get())))
                    }
                }
            }
        }
    }
}

struct ActionButton: View {
    var label: String
    var color: Color
    var action: () -> Void

    init(label: String, color: Color, action: @escaping () -> Void) {
        self.label = label
        self.color = color
        self.action = action
    }

    var body: some View {
        Button(action: action) {
            Text(label)
                .fontWeight(.bold)
                .font(.body)
                .padding(EdgeInsets(top: 10, leading: 15, bottom: 10, trailing: 15))
                .background(color)
                .cornerRadius(10)
                .foregroundColor(.white)
                .padding()
        }
    }
}

struct ContentView: View {
    @ObservedObject var model: Model

    var body: some View {
        VStack {
            Text("Crux Counter Example").font(.headline)
            Text("Rust Core, Swift Shell (SwiftUI)").padding()
            Text(String(model.view.text))
                .foregroundColor(model.view.confirmed ? Color.black : Color.gray)
                .padding()
            HStack {
                ActionButton(label: "Decrement", color: .yellow) {
                    model.update(msg: .event(.decrement))
                }
                ActionButton(label: "Increment", color: .red) {
                    model.update(msg: .event(.increment))
                }
            }
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView(model: Model())
    }
}

enum HttpError : Error {
    case generic(Error)
    case message(String)
}

func http(request: HttpRequest) async -> Result<HttpResponse, HttpError> {
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
            return .success(HttpResponse(status: status, body: body))
        } else {
            return .failure(.message("bad response"))
        }
    }
    catch {
        return .failure(.generic(error))
    }
    
}

func sse(request: SseRequest) async -> AsyncStream<Result<SseResponse, HttpError>> {
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
