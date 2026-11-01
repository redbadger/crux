import App
import Foundation

private nonisolated let logger = Log.core

/// `WeatherHandler` is the shell's side of the effect protocol: it implements
/// the generated `EffectHandler`, one method per operation the app declares,
/// each answering with the single output that operation expects. Nothing here
/// calls `resolve` — the generated `EffectDispatcher` does that, exactly as
/// often as the operation's kind says.
///
/// There is no `render` method: the generated `Core` intercepts `Render`
/// before the dispatcher sees it, so the protocol's default no-op stands.
///
/// The location and secret methods live in `location.swift` and `secret.swift`.
///
/// `EffectHandler` is `Sendable` and its requirements are not actor-isolated,
/// so the handler methods are `nonisolated`: URLSession, Keychain and
/// CoreLocation work does not belong on the main actor anyway. Where a method
/// does need main-actor state it hops, and only ever carries `Sendable`
/// values across.
@MainActor
public final class WeatherHandler {
    let httpHandler = URLSessionHttpHandler()

    /// A suite of its own, so the favourites do not share the standard defaults.
    let keyValueHandler = UserDefaultsKeyValueHandler(suiteName: "com.crux.examples.weather.store")

    let timeHandler = TaskTimeHandler()

    public init() {}
}

nonisolated extension WeatherHandler: EffectHandler {
    public func http(_ operation: HttpRequest) async -> HttpResult {
        await httpHandler.request(operation)
    }

    public func kvGet(_ operation: GetValue) async -> ValueResult {
        await keyValueHandler.get(operation)
    }

    public func kvSet(_ operation: SetValue) async -> ValueResult {
        await keyValueHandler.set(operation)
    }

    public func timeNotifyAfter(_ operation: NotifyAfter) async -> TimerId {
        await timeHandler.notifyAfter(operation)
    }

    public func timeClear(_ operation: ClearTimer) async -> TimerId {
        await timeHandler.clear(operation)
    }
}
