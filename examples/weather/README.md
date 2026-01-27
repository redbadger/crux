# Weather App Example (Crux)

This project is a cross-platform weather application example built using [Crux](https://github.com/redbadger/crux/), demonstrating a clean separation of business logic (in Rust) and platform-specific UI (iOS/SwiftUI and Android/Jetpack Compose). The app fetches weather data from the [OpenWeatherMap API](https://openweathermap.org/api) and displays it in a modern, user-friendly interface.

## Features

- Fetches current weather for a given location using OpenWeatherMap
- Add and view favorite locations
- Modern, responsive UI built with SwiftUI
- Business logic and state management in Rust, shared across platforms
- Persistent storage for favorites using Core Data (iOS)
- Cross-platform ready: core logic can be reused for other platforms (Android, Web, etc.)

## Project Structure

- `shared/` — Rust crate with domain-organized business logic: - `weather/` — Weather data fetching and processing - `location/` — Location services management - `favorites/` — Favorite locations management
  - `config.rs` — Shared configuration (API keys, endpoints)
  - `app.rs` — Core app logic and view state management
- `iOS/` — iOS app using SwiftUI, integrates with Rust via FFI
- `Android/` — Android app using Kotlin + Jetpack Compose, integrates with Rust via FFI

## Architecture Summary

- **Domain-Oriented**: Code organized by business domains (weather, location, favorites)
- **Type-Safe View States**: Enum-based workflow system for view state management
- **Crux Core**: All app logic, state, and effects are in Rust (`shared/`)
- **FFI Bridge**: `shared_types/` generates Swift (and other) bindings using UniFFI and Crux typegen
- **iOS App**: SwiftUI app (`iOS/Weather/`) calls into Rust for all business logic
- **Android App**: Compose app (`Android/`) calls into Rust for all business logic

See [ARCHITECTURE.md](ARCHITECTURE.md) for a detailed breakdown.

## Getting Started

### Prerequisites

- Rust (1.66+)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/), if targeting web
- OpenWeatherMap API key (get one at [OpenWeatherMap](https://openweathermap.org/api))

#### For iOS:

- Xcode (14+)
- Swift 5

#### For Android:

- NDK v29.0.14206865 (for Android app)

- Android Studio (for Android app)

### Setup iOS App

1. Clone the repo and navigate to this directory.

2. Set up your OpenWeatherMap API key:

   **Option 1: Environment Variable**

   ```sh
   export OPENWEATHER_API_KEY=your_api_key_here
   ```

   **Option 2: Xcode Scheme**
   1. Open `iOS/Weather.xcodeproj` in Xcode
   2. Select the Weather scheme
   3. Click Edit Scheme (⌘<)
   4. Under Run → Arguments → Environment Variables
   5. Add `OPENWEATHER_API_KEY` with your API key

3. Build the Rust shared library:

   ```sh
   cd shared
   cargo build --release
   ```

4. Generate FFI bindings:

   ```sh
   cd ../shared_types
   cargo run --release
   ```

5. Open `iOS/Weather.xcodeproj` in Xcode and run the app.

6. Set location in a running simulator:

   ```bash
   # set the location to Big Ben (Elizabeth Tower), London, UK
   xcrun simctl location booted set 51.500510810750356,-0.12462580696146475
   ```

### Setup Android App

1. Add your API key to `Android/local.properties`:

   ```properties
   OPENWEATHER_API_KEY=your_api_key_here
   ```

2. Ensure Rust targets are installed:

   ```sh
   rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
   ```

3. Run `just build` to generate kotlin bindings to core.

4. Open Android Studio, sync and run the app.

### Running Tests

- Rust logic: `cd shared && cargo test`
- iOS app: Use Xcode's test runner

## Decisions & Rationale

- **Domain-Oriented Structure**: Clear separation of concerns by business domain
- **Rust for business logic**: Ensures consistency and testability across platforms
- **Crux pattern**: Clean separation of UI and logic, easy to port to new platforms
- **UniFFI**: Automated, type-safe FFI bindings for Swift, Java, TypeScript
- **Core Data for persistence**: Native iOS persistence for favorites

## License

Apache-2.0. See [LICENSE](../LICENSE).

## Acknowledgements

- [Crux](https://github.com/redbadger/crux/)
- [OpenWeatherMap](https://openweathermap.org/)
