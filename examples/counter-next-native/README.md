# Crux Counter Example (Native Bridge)

> [!NOTE]
> This example uses `NativeBridge` with typed FFI via UniFFI annotations,
> eliminating all byte serialization between Rust and native shells.
>
> For the traditional byte-serialization approach, see [counter-next](../counter-next/README.md).

## Overview

This example demonstrates using Crux with **typed native FFI** (NativeBridge)
for native Android (Jetpack Compose) and iOS (SwiftUI) apps.

UniFFI annotations on the Rust types generate Kotlin and Swift bindings
directly -- no manual serialization, no codegen for data types, and no
intermediate byte buffers.

## Architecture

```
                    shared/ (Rust)
    ┌──────────────────────────────────────────┐
    │  App logic, UniFFI-annotated FFI types,   │
    │  NativeBridge for typed effect delivery    │
    └────────────────────┬─────────────────────┘
                         │
              UniFFI bindgen (codegen)
                   ┌─────┴─────┐
                   │           │
                   ▼           ▼
            Kotlin bindings  Swift bindings
                   │           │
                   ▼           ▼
           ┌─────────────┐ ┌──────────────┐
           │   Android    │ │     iOS      │
           │  (Jetpack    │ │  (SwiftUI)   │
           │   Compose)   │ │              │
           └─────────────┘ └──────────────┘
```

## Key Differences from counter-next

| Aspect | counter-next | counter-next-native |
|--------|--------------|---------------------|
| FFI approach | Byte serialization (Bincode) | Typed FFI (UniFFI) |
| Effect delivery | `processEffects(bytes)` | `handleEffect(NativeRequest)` |
| Resolution | `handleResponse(id, bytes)` | `resolve(id, EffectOutput)` |
| View | `view() -> Vec<u8>` | `view() -> ViewModel` |
| Shell trait | `CruxShell` | `NativeShell` |
| Type generation | facet_typegen + serde | UniFFI derives only |

## Building

### Prerequisites

- [Rust](https://rustup.rs/) (edition 2024)
- [just](https://github.com/casey/just) command runner
- For Android: Android Studio, NDK, and the
  [rust-android-gradle](https://github.com/aspect-build/rust-android-gradle) plugin
- For iOS: Xcode, [xcodegen](https://github.com/yonaskolb/XcodeGen), and
  [cargo-swift](https://github.com/nicklmcclain/cargo-swift) (v0.9.0)

### Run Tests

```sh
just test
```

### Android

Generate UniFFI Kotlin bindings and open Android Studio:

```sh
just Android/dev
```

The Gradle build (via `rust-android-gradle`) compiles the Rust shared library
for all Android architectures automatically. UniFFI-generated Kotlin bindings
are placed in `Android/generated/app/` and included as an additional source
directory.

### iOS

Build the shared Swift package and open Xcode:

```sh
just iOS/dev
```

This uses `cargo-swift` to compile the Rust library as a static `.xcframework`
and generates the UniFFI Swift bindings inside it.

## Shared Library

The shared Rust code is in `shared/`:

- `src/app.rs` -- Application logic (Event, Model, ViewModel)
- `src/ffi.rs` -- NativeBridge FFI types (EffectFfi, EffectOutput, NativeShell, CoreFFI)
- `src/capabilities/` -- Custom capabilities (SSE)
- `src/middleware.rs` -- Effect middleware (RNG)

### NativeBridge Flow

```
Shell                          Core (Rust)
  │                               │
  │  core.update(EventFfi)        │
  │──────────────────────────────>│
  │                               │  (processes event, emits effects)
  │  handleEffect(NativeRequest)  │
  │<──────────────────────────────│
  │                               │
  │  (performs side-effect, e.g.  │
  │   HTTP request)               │
  │                               │
  │  core.resolve(id, output)     │
  │──────────────────────────────>│
  │                               │  (may emit more effects)
  │  handleEffect(NativeRequest)  │
  │<──────────────────────────────│
  │                               │
  │  core.view() -> ViewModel     │
  │──────────────────────────────>│
  │<──────────────────────────────│
```

## API

The app calls a shared global counter at https://crux-counter.fly.dev:

- `GET /` -- Get current count
- `POST /inc` -- Increment
- `POST /dec` -- Decrement
- `GET /sse` -- Server-Sent Events stream

![screenshots](./counter.webp)
