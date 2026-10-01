package com.crux.examples.counter.routing

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

/// The generated `CoreBridge`, written by hand over this app's `CoreFfi`.
///
/// The codegen binary doesn't ask for the generated `FfiBridge`, because that
/// constructs `CoreFfi` with no arguments, and this app's `CoreFfi` takes the
/// `CruxShell` callback the `EffectRouter` delivers effects through. So this
/// bridge takes that callback and hands it to `CoreFfi`; the three methods
/// are bytes in, bytes out.
///
/// On native targets `update` and `resolve` always return no requests: the
/// router sends every effect it doesn't handle itself to the callback
/// instead.
///
/// `CoreFfi` holds a handle to the Rust side, so closing the bridge releases
/// the core it wraps.
class RoutingBridge(shell: CruxShell) : CoreBridge, AutoCloseable {
    private val ffi = CoreFfi(shell)

    override fun update(event: ByteArray): ByteArray = ffi.update(event)

    override fun resolve(id: UInt, output: ByteArray): ByteArray = ffi.resolve(id, output)

    override fun view(): ByteArray = ffi.view()

    override fun close() = ffi.close()
}

/// The `CruxShell` callback `CoreFfi` is constructed with.
///
/// The `EffectRouter` calls `processEffects` with the serialized requests for
/// every effect it routes to the shell. Most arrive synchronously, on the
/// thread that called `update` or `resolve` and before that call returns;
/// the rest come from the `RngHandler`'s thread, after it has answered a
/// `Random`. This forwards each batch to the generated `Core`.
///
/// The bridge needs the `CoreFfi`, the `CoreFfi` needs this callback, and the
/// callback needs the `Core`, which is built from the bridge. So `core` is
/// set once the `Core` has been constructed; nothing calls `processEffects`
/// before then, because the router only has effects once an event has been
/// sent.
///
/// @param scope where each batch is processed. `Core` is not thread-safe, so
///   the batch is posted to the main thread with `Dispatchers.Main` — not
///   `Main.immediate`, which would run it inline when the callback arrives on
///   the main thread, inside the `CoreFfi` call that triggered it. The main
///   looper runs posts in order, so the batches reach the core in the order
///   the router sent them.
class RoutedEffects(private val scope: CoroutineScope) : CruxShell {
    lateinit var core: Core

    override fun processEffects(bytes: ByteArray) {
        scope.launch(Dispatchers.Main) { core.process(bytes) }
    }
}
