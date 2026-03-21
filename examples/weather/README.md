# Weather example

This example demonstrates a full, working Crux app with multiple screens, real
API calls, persistent storage, and device capabilities like geolocation.

To keep things realistic, the app connects to a real weather API. We chose
[OpenWeatherMap](https://openweathermap.org) because it has a generous free
tier. You'll need to sign up for a free API key to run these examples.

## What you can do

- **See current weather** — the app detects your location and fetches live
  weather data (temperature, humidity, wind, clouds, visibility, sunrise/sunset)
- **Add favorites** — search for any city and save it as a favorite
- **Browse favorites** — swipe between your current location and saved cities
  (on iOS) or use the tab bar (on macOS)
- **Delete favorites** — remove saved cities with a confirmation dialog
- **Persistent storage** — favorites are saved to local storage (web) or
  key-value store (native) and restored on launch

## Architecture

The `shared` crate contains all the business logic, organised into domain
modules:

- `weather` — fetches current weather from the OpenWeatherMap API
- `location` — checks location permissions and gets the device's coordinates
- `favorites` — manages saved locations with key-value persistence
- `config` — API key configuration
- `app` — ties it all together with workflow-based navigation (Home, Favorites,
  Add Favorite)

See [ARCHITECTURE.md](ARCHITECTURE.md) for a detailed breakdown.

## Shells

- SwiftUI (iOS/macOS) — `apple/`
- Android/Kotlin — `Android/`
- Leptos — `web-leptos/`
- Next.js — `web-nextjs/`

## Setup

### 1. Get an API key

Sign up for a free API key at
[openweathermap.org/appid](https://openweathermap.org/appid).

### 2. Create `.env`

In this directory (`examples/weather/`), create a `.env` file:

```sh
export OPENWEATHER_API_KEY=your_key_here
```

### 3. Check prerequisites

```sh
just doctor
```

This verifies that the required tools are installed and that `.env` is
configured.

## Running

### Web (Leptos or Next.js)

The `.env` file is sourced automatically by the `serve` recipe:

```sh
cd web-leptos  # or web-nextjs
just serve
```

### Android

Run setup to copy the API key to `local.properties`, then open Android Studio:

```sh
just Android/setup
just Android/open
```

Build and run from Android Studio. (The `setup` step is also included in
`just Android/dev`.)

### Apple (iOS/macOS)

Generate the Xcode project (this injects the API key into the scheme) and open
it:

```sh
cd apple
just generate
just open
```

Build and run from Xcode.
