import App

/// `Core` is the shell's side of the effect protocol: it implements the
/// generated `EffectHandler`, one method per operation the app declares, each
/// answering with the single output that operation expects. Nothing here
/// calls `resolve` — the generated `EffectDispatcher` does that, exactly as
/// often as the operation's request kind says.
///
/// The methods live beside the platform code they use: `http.swift`,
/// `keyValue.swift`, `location.swift`, `secret.swift` and `time.swift`.
///
/// `EffectHandler` is `Sendable` and its requirements are not actor-isolated,
/// so the handler methods are `nonisolated`: URLSession, Keychain and
/// CoreLocation work does not belong on the main actor anyway. Where a method
/// does need main-actor state it hops, and only ever carries `Sendable`
/// values across.
nonisolated extension Core: EffectHandler {
    /// `render` is a notification, so there is nothing to answer: repaint and
    /// return.
    public func render(_: RenderOperation) {
        Task { @MainActor in refreshView() }
    }
}
