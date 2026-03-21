# Weather example

Cross-platform weather app that fetches data from the
[OpenWeatherMap API](https://openweathermap.org/api). Demonstrates HTTP
capabilities, domain-oriented code organisation, and persistent storage.

## Architecture

The `shared` directory is a crate that implements the shared crux core. It contains:

- Domain modules for `weather`, `location`, and `favorites`
- A `config.rs` for shared configuration (API keys, endpoints)
- An `app.rs` with core app logic and view-state management
- Tests that ensure events update the `Model` correctly and produce the desired
  effects.

See [ARCHITECTURE.md](ARCHITECTURE.md) for a detailed breakdown.

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

### API key

The app requires an [OpenWeatherMap](https://openweathermap.org) API key. Set it as an environment variable
before building:

```sh
export OPENWEATHER_API_KEY=your_api_key_here
```
