# Counter (HTTP) example

Builds on the [`counter`](../counter/) example by adding HTTP requests and
Server-Sent Events. The counter state lives on a shared server at
[crux-counter.fly.dev](https://crux-counter.fly.dev), so updates from one client
are visible to all connected clients.

## Architecture

The `shared` directory adds two capabilities on top of the basic counter:

- `crux_http` for GET/POST requests to the shared counter API
- A custom [SSE capability](./shared/src/sse.rs) that streams
  updates from the server
- Optimistic updates — the UI updates immediately, then reconciles when the
  server responds

## Shells

- SwiftUI (iOS/macOS) — `apple/`
- Android/Kotlin — `Android/`
- Leptos — `web-leptos/`
- NextJS — `web-nextjs/`

The Swift, Kotlin and TypeScript shells use the `Core` that type generation
emits: the codegen binary is told where `boltffi pack` puts the FFI bindings
(with `.boltffi(..)`), so each shell constructs `Core` from an
`EffectHandler` and never touches the FFI itself. The codegen binary also asks
for the handler `crux_http` ships (with `.shell_handler(&crux_http::HTTP)`), so
the shells' `http` method is one line that delegates to it.

Server-Sent Events are this app's own capability, so nothing ships a handler
for them: each shell implements the `serverSentEvents` stream method itself,
sending every chunk it reads into the `EffectSink` it is given, and then
`Done`. That is the pattern for any custom capability.

## Running

1. Choose a shell you're interested in, i.e. `apple` or `android`.
2. In the shell's directory, run `just doctor` to make sure you have the right
  tools installed
3. Run `just dev` to generate code and build that shell
4. For `apple` and `android` shells, open the IDE. For others, run `just serve`
  in the shell directory.
