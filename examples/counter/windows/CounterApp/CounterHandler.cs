using CounterApp.Shared;

namespace CounterApp;

/// <summary>
/// The shell's side of the effect protocol: the generated
/// <c>IEffectHandler</c> has one method per operation the app declares.
///
/// The counter's only effect is <c>Render</c>, and the generated <c>Core</c>
/// intercepts that before the dispatcher sees it and raises
/// <c>PropertyChanged</c> for its <c>View</c>, so the interface's default
/// no-op stands and there is nothing here to write.
/// </summary>
public sealed class CounterHandler : IEffectHandler
{
}
