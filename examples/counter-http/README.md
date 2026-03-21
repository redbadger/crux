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

## Running

1. Choose a shell you're interested in, i.e. `apple` or `Android`.
2. In the shell's directory, run `just doctor` to make sure you have the right
  tools installed
3. Run `just dev` to generate code and build that shell
4. For `apple` and `Android` shells, open the IDE. For others, run `just serve`
  in the shell directory.
