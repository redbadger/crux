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

## Getting started

### 1. Get an API key

Sign up for a free [OpenWeatherMap](https://openweathermap.org/appid) API key.

### 2. Create `.env`

In the `examples/weather/` directory, create a `.env` file:

```sh
export OPENWEATHER_API_KEY=your_key_here
```

### 3. Check prerequisites

```sh
just doctor
```

This checks that the required tools are installed and that `.env` is configured.

### 4. Run a shell

**Web shells** (Leptos or Next.js) — the `.env` is sourced automatically:

```sh
cd web-leptos  # or web-nextjs
just serve
```

**Android** — run setup first to copy the key to `local.properties`:

```sh
just Android/setup   # or just run: just Android/dev
```

Then open in Android Studio (`just Android/open`) and run.

**Apple** — the key is injected into the Xcode scheme when the project is
generated:

```sh
cd apple
just generate   # sources .env and runs xcodegen
just open       # opens in Xcode
```

Then build and run from Xcode.
