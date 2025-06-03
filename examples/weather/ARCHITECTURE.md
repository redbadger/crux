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

- **iOS App (SwiftUI)**: Handles all user interface, user input, and platform integration (e.g., Core Data for persistence).
- **Rust Core (shared/)**: Contains all business logic, state, and effect management using the Crux pattern.
- **FFI Bridge (shared_types/)**: Generates type-safe bindings for Swift (and other platforms) using UniFFI and Crux typegen.

## Data Flow
1. **User interacts with UI** (e.g., requests weather for a location).
2. **UI sends event to Rust core** via FFI (e.g., `.update(.home(.show(lat, lon)))`).
3. **Rust core updates state**, may trigger effects (HTTP, persistence, etc.).
4. **Effects are handled on the platform side** (e.g., HTTP via Swift, persistence via Core Data).
5. **Rust core returns new state/view model** to the UI for rendering.

## Key Components
- `shared/`: Rust crate with all app logic, state, and effect definitions (Crux pattern).
- `shared_types/`: Rust crate for generating FFI bindings and shared types for Swift, Java, TypeScript, etc.
- `iOS/Weather/`: SwiftUI app, integrates with Rust via generated bindings.

## Notable Design Decisions
- **Crux Pattern**: All business logic and state are in Rust, making the app portable to other platforms (Android, Web, etc.) with minimal changes.
- **UniFFI & Crux Typegen**: Automated, type-safe FFI bindings reduce boilerplate and errors.
- **Native Persistence**: Uses Core Data on iOS for storing favorites, keeping platform-specific storage idiomatic.
- **Effect System**: Side effects (HTTP, storage) are declared in Rust but executed on the platform, ensuring testability and separation of concerns.
- **Testing**: Rust logic is unit tested independently of the UI; UI can be tested with Xcode tools.

## Extending to Other Platforms
- Add a new UI (e.g., Android, Web) and generate bindings via `shared_types/`.
- Reuse the Rust core as-is, ensuring consistent logic and state across all platforms.

## Why This Architecture?
- **Consistency**: Single source of truth for business logic.
- **Testability**: Core logic is easily unit tested in Rust.
- **Portability**: Add new platforms with minimal effort.
- **Maintainability**: Clear separation of concerns.

## References
- [Crux](https://github.com/redbadger/crux/)
- [UniFFI](https://mozilla.github.io/uniffi-rs/)
- [OpenWeatherMap API](https://openweathermap.org/api) 