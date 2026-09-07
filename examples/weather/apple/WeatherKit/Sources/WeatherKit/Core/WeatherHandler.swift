import App
import Foundation

private nonisolated let logger = Log.core

/// `WeatherHandler` is the shell's side of the effect protocol: it implements
/// the generated `EffectHandler`, one method per operation the app declares,
/// each answering with the single output that operation expects. Nothing here
/// calls `resolve` — the generated `EffectDispatcher` does that, exactly as
/// often as the operation's request kind says.
///
/// There is no `render` method: the generated `Core` intercepts `Render`
/// before the dispatcher sees it, so the protocol's default no-op stands.
///
/// The methods live beside the platform code they use: `http.swift`,
/// `keyValue.swift`, `location.swift`, `secret.swift` and `time.swift`. The
/// state they share — the key-value store and the timer table — lives here.
///
/// `EffectHandler` is `Sendable` and its requirements are not actor-isolated,
/// so the handler methods are `nonisolated`: URLSession, Keychain and
/// CoreLocation work does not belong on the main actor anyway. Where a method
/// does need main-actor state it hops, and only ever carries `Sendable`
/// values across.
@MainActor
public final class WeatherHandler {
    let keyValueStore: KeyValueStore
    var activeTimers: [UInt64: Timer] = [:]

    public init() {
        do {
            keyValueStore = try KeyValueStore()
            logger.debug("KeyValueStore initialized successfully")
        } catch {
            logger.error("Failed to initialize KeyValueStore: \(error.localizedDescription)")
            fatalError("KeyValueStore initialization failed: \(error)")
        }
    }
}

nonisolated extension WeatherHandler: EffectHandler {}
