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
- **FIXME** describe the adjustments

### Two cores

## Shells

- SwiftUI (iOS/macOS) — `apple/`
- Android/Kotlin — `android/`
- Leptos — `web-leptos/`
- NextJS — `web-nextjs/`

## Running

1. Choose a shell you're interested in, i.e. `apple` or `android`.
2. In the shell's directory, run `just doctor` to make sure you have the right
   tools installed
3. Run `just dev` to generate code and build that shell
4. For `apple` and `android` shells, open the IDE. For others, run `just serve`
   in the shell directory.
