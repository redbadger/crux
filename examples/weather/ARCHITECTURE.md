# Architecture Overview

This project demonstrates a cross-platform architecture using [Crux](https://github.com/redbadger/crux/) to separate business logic (Rust) from platform-specific UI (SwiftUI for iOS). The architecture is designed for maximum code reuse, testability, and maintainability.

## High-Level Structure

```
+-------------------+      +-------------------+
|   iOS App (UI)    | <--> |   Rust Core (Crux) |
+-------------------+      +-------------------+
         |                          |
         |        FFI (UniFFI)      |
         +--------------------------+
```

- **iOS App (SwiftUI)**: Handles all user interface, user input, and platform integration (e.g., Core Data for persistence, CoreLocation for location services).
- **Rust Core (shared/)**: Contains all business logic, state, and effect management using the Crux pattern.
- **FFI Bridge (shared_types/)**: Generates type-safe bindings for Swift (and other platforms) using UniFFI and Crux typegen.

## Data Flow
1. **User interacts with UI** (e.g., opens the app, requests weather for a location).
2. **UI sends event to Rust core** via FFI (e.g., `.update(.home(.show))`).
3. **Rust core updates state**, may trigger effects (HTTP, persistence, location, etc.).
4. **Effects are handled on the platform side** (e.g., HTTP via Swift, persistence via Core Data, location via CoreLocation).
5. **Rust core returns new state/view model** to the UI for rendering.

## Key Components
- `shared/`: Rust crate with all app logic, state, and effect definitions (Crux pattern).
- `shared_types/`: Rust crate for generating FFI bindings and shared types for Swift, Java, TypeScript, etc.
- `iOS/Weather/`: SwiftUI app, integrates with Rust via generated bindings.

## Effect System

The app uses several cross-platform effects that are declared in Rust but implemented natively on each platform:

### Location Effect (`shared/src/effects/location.rs`)
- **Operations**: Check if location services are enabled, get current location coordinates
- **Cross-platform**: Rust defines the interface, iOS implements using CoreLocation
- **Features**: Permission handling, timeout management, error handling
- **Integration**: Automatically fetches weather for current location on app startup

### HTTP Effect 
- **Purpose**: API calls to OpenWeatherMap for weather data and geocoding
- **Implementation**: Native HTTP clients on each platform

### Key-Value Storage Effect
- **Purpose**: Persistent storage for user favorites
- **Implementation**: Core Data on iOS, can be adapted for other platforms

## Location-Based Weather Flow

1. **App Launch**: Home screen triggers location permission check
2. **Permission Check**: `LocationOperation::IsLocationEnabled` queries platform location services
3. **Location Fetch**: If enabled, `LocationOperation::GetLocation` retrieves current coordinates
4. **Weather Request**: Location coordinates automatically trigger weather API call
5. **UI Update**: Weather data for current location is displayed

## Notable Design Decisions
- **Crux Pattern**: All business logic and state are in Rust, making the app portable to other platforms (Android, Web, etc.) with minimal changes.
- **UniFFI & Crux Typegen**: Automated, type-safe FFI bindings reduce boilerplate and errors.
- **Native Platform APIs**: Uses CoreLocation on iOS for location services, keeping platform-specific integrations idiomatic.
- **Effect System**: Side effects (HTTP, storage, location) are declared in Rust but executed on the platform, ensuring testability and separation of concerns.
- **Automatic Location Weather**: App intelligently fetches weather for user's current location when available and permitted.
- **Permission Handling**: Platform-native permission flows with graceful fallbacks when location is unavailable.
- **Testing**: Rust logic is unit tested independently of the UI; UI can be tested with Xcode tools.

## Platform-Specific Implementation

### iOS Location Integration
- **Permissions**: Uses `NSLocationWhenInUseUsageDescription` in Info.plist
- **Native APIs**: Implements location services using `CLLocationManager`
- **Error Handling**: Comprehensive error handling for denied permissions, disabled services, and timeouts
- **Threading**: Async/await pattern with proper main thread coordination

## Extending to Other Platforms
- Add a new UI (e.g., Android, Web) and generate bindings via `shared_types/`.
- Implement location services using platform-native APIs (Android LocationManager, Web Geolocation API).
- Reuse the Rust core as-is, ensuring consistent logic and state across all platforms.

## Why This Architecture?
- **Consistency**: Single source of truth for business logic across all platforms.
- **Native Integration**: Platform-specific effects use native APIs for best user experience.
- **Testability**: Core logic is easily unit tested in Rust, effects can be mocked.
- **Portability**: Add new platforms with minimal effort, only implementing platform-specific effects.
- **Maintainability**: Clear separation of concerns between business logic and platform integration.

## References
- [Crux](https://github.com/redbadger/crux/)
- [UniFFI](https://mozilla.github.io/uniffi-rs/)
- [OpenWeatherMap API](https://openweathermap.org/api) 
- [iOS CoreLocation](https://developer.apple.com/documentation/corelocation) 