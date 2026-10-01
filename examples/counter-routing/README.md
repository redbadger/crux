# Counter (Routing) example

Builds on [`counter-http`](../counter-http/) by adding a "I'm feeling lucky"
button that adjusts the counter by a random amount between -5 and 5. The random
number generation is handled by
[routing](../../docs/src/rfcs/effect-router.md) — Rust code that routes the random
effects to a core-side implementation.

## Architecture

The `shared` directory adds a custom
[`Random` capability](./shared/src/capabilities/) on top of the HTTP counter.
This demonstrates:

- Defining a custom `Operation` (request/response types)
- Wrapping the core in an `EffectRouter` (in `shared/src/ffi.rs`), whose
  routing closure sends `Random` to a core-side `RngHandler` and serializes
  every other effect for the shell
- Resolving the routed request from the handler's own thread, through the
  `ResolveSink` the router implements

### Two cores

- **BoltFFI native shells** — The `EffectRouter` handles `Random` entirely in
  Rust. Every other effect reaches the shell through the `CruxShell` callback,
  including those produced inside `update` and `resolve`, which therefore
  return no requests of their own.
- **Web compatibility shell** — The `RngHandler` needs a thread, so on wasm the
  core is bridged directly, `Random` effects pass through to the shell, and
  the shell answers them in JavaScript.

## Shells

- SwiftUI (iOS/macOS) — `apple/`
- Android/Kotlin — `Android/`
- Leptos — `web-leptos/`
- NextJS — `web-nextjs/`

The Swift, Kotlin and TypeScript shells use the `Core` that type generation
emits, with an `EffectHandler` per shell. The codegen binary asks for the
handler `crux_http` ships (with `.shell_handler(&crux_http::HTTP)`), so each
shell's `http` method is one line that delegates to it. Server-Sent Events are
this app's own capability, so each shell implements the `serverSentEvents`
stream method itself, sending every chunk it reads into the `EffectSink` it is
given, and then `Done`.

The codegen binary doesn't configure `.boltffi(..)`, because `CoreFfi::new`
takes the shell's callback and the generated `FfiBridge` constructs `CoreFfi`
with no arguments. So each shell writes a three-method `CoreBridge` over its
own `CoreFfi` (`RoutingBridge`), builds the generated `Core` from it, and
forwards the callback's bytes to the `Core`'s `process`. The callback can
arrive while `update` is still running, so the Swift and Kotlin shells always
defer to the main thread: Swift through an `AsyncStream` read by one task on
the main actor, and Kotlin by posting with `Dispatchers.Main` rather than
`Main.immediate`. The middleware example's
[README](../counter-middleware/README.md) has the details.

## Running

1. Choose a shell you're interested in, i.e. `apple` or `android`.
2. In the shell's directory, run `just doctor` to make sure you have the right
   tools installed
3. Run `just dev` to generate code and build that shell
4. For `apple` and `android` shells, open the IDE. For others, run `just serve`
   in the shell directory.
