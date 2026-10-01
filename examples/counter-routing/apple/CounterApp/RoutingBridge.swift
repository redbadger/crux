import App
import Foundation
import Shared

/// The generated `CoreBridge`, written by hand over this app's `CoreFfi`.
///
/// The codegen binary doesn't ask for the generated `FfiBridge`, because that
/// constructs `CoreFfi` with no arguments, and this app's `CoreFfi` takes the
/// `CruxShell` callback the `EffectRouter` delivers effects through. So this
/// bridge takes that callback and hands it to `CoreFfi`; the three methods are
/// bytes in, bytes out.
///
/// On native targets `update` and `resolve` always return no requests: the
/// router sends every effect it doesn't handle itself to the callback
/// instead.
///
/// `@unchecked Sendable` because `CoreFfi` is a class Swift cannot prove
/// `Sendable`; its state is a handle into a mutex-guarded bridge on the Rust
/// side, so calling it from any task is safe.
struct RoutingBridge: CoreBridge, @unchecked Sendable {
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

/// The `CruxShell` callback `CoreFfi` is constructed with.
///
/// The `EffectRouter` calls `processEffects` with the serialized requests for
/// every effect it routes to the shell. Most arrive synchronously, on the
/// thread that called `update` or `resolve` and before that call returns; the
/// rest come from the `RngHandler`'s thread, after it has answered a `Random`.
/// The callback doesn't hold the `Core` — it can't, because the `Core` is
/// built from the bridge, which is built from this — so it puts the bytes on
/// a stream, in the order they arrive, for `makeCore` to forward.
final class RoutedEffects: CruxShell, Sendable {
    let stream: AsyncStream<[UInt8]>
    private let continuation: AsyncStream<[UInt8]>.Continuation

    init() {
        (stream, continuation) = AsyncStream.makeStream(of: [UInt8].self)
    }

    func processEffects(bytes: Data) {
        continuation.yield([UInt8](bytes))
    }
}

/// Build the generated `Core` over the routing bridge.
///
/// Construction runs one way — callback, then bridge, then `Core` — and the
/// stream carries the callback's bytes back the other way. One task on the
/// main actor reads it and passes each batch to `Core.process(bytes:)`, which
/// has to run there because `Core` is `@MainActor`. A single reader keeps the
/// batches in the order the router sent them, and because it runs later, a
/// batch sent from inside `update` is never processed while `update` is
/// still running.
@MainActor
func makeCore() -> Core {
    let effects = RoutedEffects()
    let core = Core(bridge: RoutingBridge(shell: effects), handler: CounterHandler())

    let stream = effects.stream
    Task { [weak core] in
        for await bytes in stream {
            core?.process(bytes: bytes)
        }
    }

    return core
}
