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
- Android/Kotlin — `android/`
- Leptos — `web-leptos/`
- Next.js — `web-nextjs/`

## Setup

### 1. Get an API key

Sign up for a free API key at
[openweathermap.org/appid](https://openweathermap.org/appid). The app will
prompt you to enter it on first launch.

### 2. Check prerequisites

```sh
just doctor
```

This verifies that the required tools are installed.

## Running

### Web (Leptos or Next.js)

```sh
cd web-leptos  # or web-nextjs
just serve
```

### Android

```sh
cd Android
just open
```

Build and run from Android Studio.

### Apple (iOS/macOS)

Generate the Xcode project and open it:

```sh
cd apple
just generate
just open
```

Build and run from Xcode.
