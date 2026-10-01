import App

/// `CounterHandler` is the shell's side of the effect protocol: it implements
/// the generated `EffectHandler`, which has one method per operation the app
/// declares.
///
/// The counter's only effect is `Render`, and the generated `Core` intercepts
/// that before the dispatcher sees it, so the protocol's default no-op stands
/// and there is nothing here to write.
struct CounterHandler: EffectHandler {}
