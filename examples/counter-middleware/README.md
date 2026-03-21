# Counter (Middleware) example

Builds on [`counter-http`](../counter-http/) by adding a custom capability that
delegates work to the shell. A "Random" button asks the shell for a random
number between -5 and 5, then adjusts the counter by that amount.

## Architecture

The `shared` directory adds a custom
[`Random` capability](./shared/src/capabilities/) on top of the HTTP counter.
This demonstrates:

- Defining a custom capability with its own request/response types
- Shell-side middleware — the shell generates the random number and sends it
  back to the core
- Using `Command::request_from_shell` to round-trip through the shell

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
