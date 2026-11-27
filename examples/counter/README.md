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
- Android/Kotlin — `Android/`
- WinUI3 / C# (Windows, .NET 10) — `windows/`
- Leptos — `web-leptos/`
- NextJS — `web-nextjs/`
- Yew — `web-yew/`
- Dioxus — `web-dioxus/`
- React Router — `web-react-router/`
- Tauri — `tauri/`
- TUI (ratatui) — `tui/`

## Running

1. Choose a shell you're interested in, i.e. `apple` or `Android`.
2. In the shell's directory, run `just doctor` to make sure you have the right
  tools installed
3. Run `just dev` to generate code and build that shell
4. For `apple`, `Android`, and `windows` shells, open the IDE (Xcode,
  Android Studio, or Visual Studio). For `tui`, run `just run`. For others,
  run `just serve` in the shell directory.

### On Windows

Windows support is partial:

- `windows` (WinUI3) is covered by CI.
- `Android` and `tauri` have Windows-specific recipes. CI builds both on
  Windows.
- `shared` and `tui` set `windows-shell`, and their recipes are plain `cargo`
  commands. They are untested on Windows.
- `web-leptos` and `web-yew` have no Windows-specific recipes, but nothing
  POSIX-only in them either. just's default shell is `sh`, so they can work
  where Git Bash or a similar shell is on `PATH`.
- `web-dioxus`, `web-nextjs` and `web-react-router` use `rm -rf`, so they need
  porting before they will run.

The ported directories set `windows-shell` so their recipes run through
`powershell.exe`, which ships with Windows. You do not need PowerShell 7.
The top-level `examples/Justfile` also runs the other examples, which do not
set `windows-shell` yet.

Gradle and pnpm both create paths long enough to hit the 260-character
`MAX_PATH` limit, so enable long path support before building the `Android`
shell, from an elevated prompt:

```powershell
Set-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem' `
  -Name LongPathsEnabled -Value 1
```

Run `just doctor` in the shell directory first. It reports each missing tool
with the command that installs it.
