package com.crux.example.weather.core

import com.crux.example.weather.CoreBridge
import com.crux.example.weather.CoreFfi
import javax.inject.Inject
import javax.inject.Singleton

/// The generated `CoreBridge`, implemented over BoltFFI's `CoreFfi`: bytes in,
/// bytes out, nothing else. This is the only file in the app that knows the
/// Rust core exists — swap it for a fake and the rest of the shell is
/// unchanged.
@Singleton
class LiveBridge
    @Inject
    constructor() : CoreBridge {
        private val coreFfi = CoreFfi()

        override fun update(event: ByteArray): ByteArray = coreFfi.update(event)

        override fun resolve(
            id: UInt,
            output: ByteArray,
        ): ByteArray = coreFfi.resolve(id, output)

        override fun view(): ByteArray = coreFfi.view()
    }
