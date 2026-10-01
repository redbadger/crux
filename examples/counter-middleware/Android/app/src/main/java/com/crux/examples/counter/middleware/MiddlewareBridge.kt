package com.crux.examples.counter.middleware

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

/// The generated `CoreBridge`, written by hand over this app's `CoreFfi`.
///
/// The codegen binary doesn't ask for the generated `FfiBridge`, because that
/// constructs `CoreFfi` with no arguments, and this app's `CoreFfi` takes the
/// `CruxShell` callback the middleware uses to deliver effects after `update`
/// or `resolve` has returned. So this bridge takes that callback and hands it
/// to `CoreFfi`; the three methods are bytes in, bytes out.
///
/// `CoreFfi` holds a handle to the Rust side, so closing the bridge releases
/// the core it wraps.
class MiddlewareBridge(shell: CruxShell) : CoreBridge, AutoCloseable {
    private val ffi = CoreFfi(shell)

    override fun update(event: ByteArray): ByteArray = ffi.update(event)

    override fun resolve(id: UInt, output: ByteArray): ByteArray = ffi.resolve(id, output)

    override fun view(): ByteArray = ffi.view()

    override fun close() = ffi.close()
}

/// The `CruxShell` callback `CoreFfi` is constructed with.
///
/// The middleware calls `processEffects` from its own thread, with the
/// serialized requests the app made after the middleware resolved one of its
/// effects, and this forwards them to the generated `Core`.
///
/// The bridge needs the `CoreFfi`, the `CoreFfi` needs this callback, and the
/// callback needs the `Core`, which is built from the bridge. So `core` is
/// set once the `Core` has been constructed; nothing calls `processEffects`
/// before then, because the middleware only has work once an event has been
/// sent.
///
/// @param scope where each batch is processed. `Core` is not thread-safe, so
///   the batch is posted to the main thread with `Dispatchers.Main` — not
///   `Main.immediate`, which would run it inline when the callback arrives on
///   the main thread, inside the `CoreFfi` call that triggered it. The main
///   looper runs posts in order, so the batches reach the core in the order
///   the middleware sent them.
class MiddlewareEffects(private val scope: CoroutineScope) : CruxShell {
    lateinit var core: Core

    override fun processEffects(bytes: ByteArray) {
        scope.launch(Dispatchers.Main) { core.process(bytes) }
    }
}
