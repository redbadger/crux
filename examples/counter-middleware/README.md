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

- **Native (uniffi)** — The `RngMiddleware` intercepts `Random` effects and
  handles them entirely in Rust. The shell never sees them.
- **Web (wasm_bindgen)** — Middleware can't run in wasm (it uses
  `std::thread::spawn`), so `Random` effects pass through to the shell, which
  handles them in JavaScript. This demonstrates the app working, but not the
  middleware feature itself.

## Shells

- SwiftUI (iOS/macOS) — `apple/`
- Android/Kotlin — `Android/`
- Leptos — `web-leptos/`
- NextJS — `web-nextjs/`

## Running

1. Choose a shell you're interested in, i.e. `apple` or `Android`.
2. In the shell's directory, run `just doctor` to make sure you have the right
   tools installed
3. Run `just dev` to generate code and build that shell
4. For `apple` and `Android` shells, open the IDE. For others, run `just serve`
   in the shell directory.
