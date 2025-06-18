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

## Domain-Oriented Structure
The codebase follows a domain-oriented approach with the following key domains:

### Weather Domain
- **Purpose**: Handles weather data fetching and processing
- **Location**: `shared/src/weather/`
- **Components**: 
  - `model/`: Weather data structures and response types
  - `events.rs`: Weather-related events

### Location Domain
- **Purpose**: Manages location services and coordinates
- **Location**: `shared/src/location/`
- **Components**:
  - `effect.rs`: Location service operations
  - `model/`: Location data structures and response types
  - `events.rs`: Location-related events

### Favorites Domain
- **Purpose**: Manages user's favorite locations
- **Location**: `shared/src/favorites/`
- **Components**:
  - `model.rs`: Favorite location structures
  - `events.rs`: Favorites management events (add, delete, list operations)

## View State Management
The app uses a workflow-based approach for managing view state and data presentation (`app.rs`):

### Workflow Enum
- Defines distinct UI states: `Home`, `Favorites`, `AddFavorite`
- Each state corresponds to a specific view with its data requirements
- Handles navigation between different views

### ViewModel Structure
- Segregates data based on current workflow state
- Provides type-safe view models for each workflow variant
- Ensures UI only receives data relevant to current view

This approach maintains clean separation between domain logic and view state while providing type-safe navigation and data presentation.

## Data Flow
1. **User interacts with UI** (e.g., opens the app, requests weather for a location).
2. **UI sends event to Rust core** via FFI (e.g., `.update(.home(.show))`).
3. **Rust core updates state**, may trigger effects (HTTP, persistence, location, etc.).
4. **Effects are handled on the platform side** (e.g., HTTP via Swift, persistence via Core Data, location via CoreLocation).
5. **Rust core returns new state/view model** to the UI for rendering.

## Key Components
- `shared/`: Rust crate with domain-organized logic, state, and effect definitions
- `shared_types/`: Rust crate for generating FFI bindings and shared types
- `iOS/Weather/`: SwiftUI app, integrates with Rust via generated bindings

## Effect System

The app uses several cross-platform effects that are declared in Rust but implemented natively on each platform:

### Location Effect (`shared/src/location/effect.rs`)
- **Operations**: Check if location services are enabled, get current location coordinates
- **Cross-platform**: Rust defines the interface, iOS implements using CoreLocation
- **Features**: Permission handling, timeout management, error handling
- **Integration**: Automatically fetches weather for current location on app startup

### HTTP Effect 
- **Purpose**: API calls to OpenWeatherMap for weather data and geocoding
- **Implementation**: Native HTTP clients on each platform
- **Configuration**: API key managed in `shared/src/config.rs`

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
- **Domain-Oriented Structure**: Code organized by business domains for better maintainability
- **Crux Pattern**: All business logic and state are in Rust
- **UniFFI & Crux Typegen**: Automated, type-safe FFI bindings
- **Centralized Configuration**: API keys and endpoints managed in `shared/src/config.rs`
- **Native Platform APIs**: Uses CoreLocation on iOS for location services
- **Effect System**: Side effects declared in Rust but executed on platform
- **Testing**: Comprehensive testing at domain level

## Platform-Specific Implementation

### iOS Location Integration
- **Permissions**: Uses `NSLocationWhenInUseUsageDescription` in Info.plist
- **Native APIs**: Implements location services using `CLLocationManager`
- **Error Handling**: Comprehensive error handling for denied permissions, disabled services, and timeouts
- **Threading**: Async/await pattern with proper main thread coordination

## Extending to Other Platforms
- Add a new UI (e.g., Android, Web) and generate bindings via `shared_types/`.
- Implement location services using platform-native APIs.
- Reuse the Rust core as-is, ensuring consistent logic and state across all platforms.

## Why This Architecture?
- **Domain Separation**: Clear boundaries between different parts of the application
- **Consistency**: Single source of truth for business logic across all platforms
- **Native Integration**: Platform-specific effects use native APIs for best user experience
- **Testability**: Core logic is easily unit tested in Rust, effects can be mocked
- **Portability**: Add new platforms with minimal effort
- **Maintainability**: Clear separation of concerns between domains

## References
- [Crux](https://github.com/redbadger/crux/)
- [UniFFI](https://mozilla.github.io/uniffi-rs/)
- [OpenWeatherMap API](https://openweathermap.org/api) 
- [iOS CoreLocation](https://developer.apple.com/documentation/corelocation) 