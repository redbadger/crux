# Counter example

Simple counter example, with tests. This is the starting point for understanding
Crux.

## Architecture

The `shared` directory is a crate that implements the shared crux core. It contains:

- An `Event` with three variants: `Increment`, `Decrement` and `Reset`
- A `Model` with a `count` field
- Tests that ensure events update the `Model` correctly and produce the desired
  effects.

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

## Running

1. Choose a shell you're interested in, i.e. `apple` or `android`.
2. In the shell's directory, run `just doctor` to make sure you have the right
  tools installed
3. Run `just dev` to generate code and build that shell
4. For `apple`, `android`, and `windows` shells, open the IDE (Xcode,
  Android Studio, or Visual Studio). For `tui`, run `just run`. For others,
  run `just serve` in the shell directory.
