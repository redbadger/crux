# Counter (Middleware) example

Builds on [`counter-http`](../counter-http/) by adding a "I'm feeling lucky"
button that adjusts the counter by a random amount between -5 and 5. The random
number generation is handled by
[middleware](../../docs/src/part-3/middleware.md) — Rust code that intercepts
effects before they reach the shell.

## Architecture

The `shared` directory adds a custom
[`Random` capability](./shared/src/capabilities/) on top of the HTTP counter.
This demonstrates:

- Defining a custom `Operation` (request/response types) for the middleware
- Implementing `EffectMiddleware` to handle the operation in Rust
- Wiring middleware into the core with `.handle_effects_using()`
- Narrowing the `Effect` enum with `.map_effect()` so the shell never sees
  `Random` effects

### Two cores

This example has two FFI bridges that wire up the core differently:

- **BoltFFI native shells** — The `RngMiddleware` intercepts `Random` effects and
  handles them entirely in Rust. The shell never sees them.
- **Web compatibility shell** — Middleware can't run in the current wasm path (it uses
  `std::thread::spawn`), so `Random` effects pass through to the shell, which
  handles them in JavaScript. This demonstrates the app working, but not the
  middleware feature itself.

## Shells

- SwiftUI (iOS/macOS): `apple/`
- Android/Kotlin: `Android/`
- Leptos: `web-leptos/`
- NextJS: `web-nextjs/`

The Swift, Kotlin and TypeScript shells use the `Core` that type generation
emits, with an `EffectHandler` per shell. The codegen binary asks for the
handler `crux_http` ships (with `.shell_handler(&crux_http::HTTP)`), so each
shell's `http` method is one line that delegates to it. Server-Sent Events are
this app's own capability, so each shell implements the `serverSentEvents`
stream method itself, sending every chunk it reads into the `EffectSink` it is
given, and then `Done`.

Unlike [`counter-http`](../counter-http/), the codegen binary doesn't
configure `.boltffi(..)`: `CoreFfi::new` takes the shell's `CruxShell`
callback, which the middleware uses to deliver effects after `update` or
`resolve` has returned, and the generated `FfiBridge` constructs `CoreFfi`
with no arguments. So each shell writes a three-method `CoreBridge` over its
own `CoreFfi` (`MiddlewareBridge`), builds the generated `Core` from it, and
forwards the callback's bytes to the `Core`'s `process`:

- **Swift**: the callback puts the bytes on an `AsyncStream`, and one task on
  the main actor feeds them to `Core.process(bytes:)` (see `makeCore()`).
- **Kotlin**: the callback is given the `Core` once it has been constructed,
  and posts each batch to the main thread with `Dispatchers.Main`.
- **TypeScript**: the callback closes over the `Core`. On wasm there is no
  middleware, so it is never called, and the handler answers `random` itself.

The native shells' handlers still have a `random` method, because type
generation follows the app's `Effect`, but it is never called.

## Running

1. Choose a shell you're interested in, i.e. `apple` or `android`.
2. In the shell's directory, run `just doctor` to make sure you have the right
   tools installed
3. Run `just dev` to generate code and build that shell
4. For `apple` and `android` shells, open the IDE. For others, run `just serve`
   in the shell directory.
