package com.crux.examples.counter

/// The shell's side of the effect protocol: the generated `EffectHandler` has
/// one method per operation the app declares.
///
/// The counter's only effect is `Render`, and the generated `Core` intercepts
/// that before the dispatcher sees it and refreshes its `view` flow, so the
/// interface's default no-op stands and there is nothing here to write.
object CounterHandler : EffectHandler
