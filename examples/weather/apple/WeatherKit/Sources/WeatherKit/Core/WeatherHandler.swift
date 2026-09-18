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
/// HTTP, the store and timers are `crux_http`'s, `crux_kv`'s and `crux_time`'s
/// own business, so this shell registers the handlers those crates ship and
/// delegates to them — one line per operation, below — rather than writing
/// their rules out again. The rest live beside the platform code they use:
/// `location.swift` and `secret.swift`.
///
/// `EffectHandler` is `Sendable` and its requirements are not actor-isolated,
/// so the handler methods are `nonisolated`: URLSession, Keychain and
/// CoreLocation work does not belong on the main actor anyway. Where a method
/// does need main-actor state it hops, and only ever carries `Sendable`
/// values across.
@MainActor
public final class WeatherHandler {
    /// The shipped `crux_http` handler, over the shared `URLSession`. A shell
    /// that needed a pinned session would write
    /// `URLSessionHttpHandler(session:)` here instead.
    let httpHandler = URLSessionHttpHandler.shared

    /// The shipped `crux_kv` handler, over a suite of its own so that the
    /// app's favourites do not share a namespace with the standard defaults.
    let keyValueHandler = UserDefaultsKeyValueHandler(suiteName: "com.crux.examples.weather.store")

    /// The shipped `crux_time` handler. It owns the timer table, so there is
    /// one of it, made here.
    let timeHandler = TaskTimeHandler()

    public init() {}
}

nonisolated extension WeatherHandler: EffectHandler {
    /// `Http` is a request: the shell performs it and answers with exactly one
    /// `HttpResult`. The rules for producing one — which `URLError` is a
    /// timeout, that a 404 is an answer and not a failure — are the shipped
    /// handler's.
    public func http(_ operation: HttpRequest) async -> HttpResult {
        await httpHandler.request(operation)
    }

    /// `KvGet` is answered with a `ValueResult` — the value stored under the
    /// key, or the error that stopped us reading it.
    public func kvGet(_ operation: App.Get) async -> ValueResult {
        await keyValueHandler.get(operation)
    }

    /// `KvSet` is answered with the value it replaced.
    public func kvSet(_ operation: App.Set) async -> ValueResult {
        await keyValueHandler.set(operation)
    }

    /// `TimeNotifyAfter` is answered exactly once, with the id of the timer
    /// that fired — and, if `timeClear` got there first, harmlessly late.
    public func timeNotifyAfter(_ operation: NotifyAfter) async -> TimerId {
        await timeHandler.notifyAfter(operation)
    }

    /// `TimeClear` cancels the timer and answers with the id it named.
    public func timeClear(_ operation: Clear) async -> TimerId {
        await timeHandler.clear(operation)
    }
}
