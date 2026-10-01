import App
import Foundation
import Shared

// ANCHOR: bridge
/// The generated `CoreBridge`, written by hand over this app's `CoreFfi`.
///
/// The codegen binary doesn't ask for the generated `FfiBridge`, because that
/// constructs `CoreFfi` with no arguments, and this app's `CoreFfi` takes the
/// `CruxShell` callback the middleware uses to deliver effects after `update`
/// or `resolve` has returned. So this bridge takes that callback and hands it
/// to `CoreFfi`; the three methods are bytes in, bytes out.
///
/// `@unchecked Sendable` because `CoreFfi` is a class Swift cannot prove
/// `Sendable`; its state is a handle into a mutex-guarded bridge on the Rust
/// side, so calling it from any task is safe.
struct MiddlewareBridge: CoreBridge, @unchecked Sendable {
    private let ffi: CoreFfi

    init(shell: CruxShell) {
        ffi = CoreFfi(shell: shell)
    }

    func update(_ event: [UInt8]) -> [UInt8] {
        [UInt8](ffi.update(data: Data(event)))
    }

    func resolve(_ id: UInt32, _ output: [UInt8]) -> [UInt8] {
        [UInt8](ffi.resolve(effectId: id, data: Data(output)))
    }

    func view() -> [UInt8] {
        [UInt8](ffi.view())
    }
}
// ANCHOR_END: bridge

// ANCHOR: callback
/// The `CruxShell` callback `CoreFfi` is constructed with.
///
/// The middleware calls `processEffects` from its own thread, with the
/// serialized requests the app made after the middleware resolved one of its
/// effects. The callback doesn't hold the `Core` (it can't, because the
/// `Core` is built from the bridge, which is built from this), so it puts the
/// bytes on a stream, in the order they arrive, for `makeCore` to forward.
final class MiddlewareEffects: CruxShell, Sendable {
    let stream: AsyncStream<[UInt8]>
    private let continuation: AsyncStream<[UInt8]>.Continuation

    init() {
        (stream, continuation) = AsyncStream.makeStream(of: [UInt8].self)
    }

    func processEffects(bytes: Data) {
        continuation.yield([UInt8](bytes))
    }
}
// ANCHOR_END: callback

// ANCHOR: make_core
/// Build the generated `Core` over the middleware bridge.
///
/// Construction runs one way (callback, then bridge, then `Core`), and the
/// stream carries the callback's bytes back the other way. One task on the
/// main actor reads it and passes each batch to `Core.process(bytes:)`, which
/// has to run there because `Core` is `@MainActor`. A single reader keeps the
/// batches in the order the middleware sent them.
@MainActor
func makeCore() -> Core {
    let effects = MiddlewareEffects()
    let core = Core(bridge: MiddlewareBridge(shell: effects), handler: CounterHandler())

    let stream = effects.stream
    Task { [weak core] in
        for await bytes in stream {
            core?.process(bytes: bytes)
        }
    }

    return core
}
// ANCHOR_END: make_core
