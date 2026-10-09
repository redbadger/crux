# Counter example

Simple counter example, with tests. This is the starting point for understanding
Crux.

## Architecture

The `shared` directory is a crate that implements the shared crux core. It contains:

- An `Event` with three variants: `Increment`, `Decrement` and `Reset`
- A `Model` with a `count` field
- Tests that ensure events update the `Model` correctly and produce the desired
  effects.

The `ffi` directory is a crate (`shared_ffi`) that exports the core to the
Swift, Kotlin, TypeScript and C# shells through BoltFFI, and holds the `codegen`
binary that generates their types. The Rust shells depend on `shared` directly.

## Shells

- SwiftUI (iOS/macOS) — `apple/`
- Android/Kotlin — `android/`
- WinUI3 / C# (Windows, .NET 10) — `windows/`
- Leptos — `web-leptos/`
- NextJS — `web-nextjs/`
- Yew — `web-yew/`
- Dioxus — `web-dioxus/`
- React Router — `web-react-router/`
- Tauri — `tauri/`
- TUI (ratatui) — `tui/`

The Swift, Kotlin, TypeScript and C# shells use the `Core` that type
generation emits: the codegen binary is told where `boltffi pack` puts the FFI
bindings (with `.boltffi(..)`), so each shell constructs `Core` from an
`EffectHandler` and never touches the FFI itself. Counter's only effect is
`Render`, which the generated `Core` handles, so the handlers are empty.

For the hand-written version of these shells, which wraps `CoreFfi`
directly as Part I of the book walks through, see
[`counter-tutorial`](../counter-tutorial).

## Running

1. Choose a shell you're interested in, i.e. `apple` or `android`.
2. In the shell's directory, run `just doctor` to make sure you have the right
  tools installed
3. Run `just dev` to generate code and build that shell
4. For `apple`, `android`, and `windows` shells, open the IDE (Xcode,
  Android Studio, or Visual Studio). For `tui`, run `just run`. For others,
  run `just serve` in the shell directory.
