using System;
using System.Collections.Concurrent;
using System.Threading;
using System.Threading.Tasks;

/// <summary>
/// The shell side of <c>crux_time</c>, as one method per operation.
/// </summary>
/// <remarks>
/// <para>
/// Each method's shape is the generated <c>IEffectHandler</c>'s, so an app's
/// handler holds an instance and delegates in one line:
/// </para>
/// <code>
/// public sealed class MyHandler : IEffectHandler
/// {
///     private readonly ITimeHandler _time = new TaskTimeHandler();
///
///     public Task&lt;TimerId&gt; TimeNotifyAfter(NotifyAfter operation) => _time.NotifyAfter(operation);
///     public Task&lt;TimerId&gt; TimeClear(ClearTimer operation) => _time.Clear(operation);
/// }
/// </code>
/// <para>
/// <c>NotifyAt</c> and <c>NotifyAfter</c> are answered exactly once, with the
/// timer's own id, when it fires. <c>Clear</c> cancels the timer it names and is
/// answered with the same id.
/// </para>
/// <para>
/// A cleared timer's <c>NotifyAt</c> or <c>NotifyAfter</c> may still answer — a
/// shell need not race the two. By then the core has stopped listening for that
/// answer and ignores it, which is why the implementation below completes the
/// pending task rather than leaving it running for the process's lifetime.
/// </para>
/// </remarks>
public interface ITimeHandler
{
    /// <summary>The current wall-clock time.</summary>
    Task<Instant> Now(Now operation);

    /// <summary>Answer with <c>operation.Id</c> once <c>operation.Instant</c> has arrived.</summary>
    Task<TimerId> NotifyAt(NotifyAt operation);

    /// <summary>Answer with <c>operation.Id</c> once <c>operation.Duration</c> has elapsed.</summary>
    Task<TimerId> NotifyAfter(NotifyAfter operation);

    /// <summary>Cancel the timer <c>operation.Id</c> names, and answer with it.</summary>
    Task<TimerId> Clear(ClearTimer operation);
}

/// <summary>
/// An <see cref="ITimeHandler"/> whose timers are <c>Task.Delay</c>s.
/// </summary>
/// <remarks>
/// The timer table is state, so the app constructs one handler where it
/// constructs its own.
/// </remarks>
public sealed class TaskTimeHandler : ITimeHandler
{
    private static readonly DateTime Epoch = new(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc);

    /// <summary>The timers that have not fired or been cleared, by id.</summary>
    private readonly ConcurrentDictionary<ulong, CancellationTokenSource> _timers = new();

    public Task<Instant> Now(Now operation)
    {
        var elapsed = DateTime.UtcNow - Epoch;
        var seconds = (ulong)elapsed.TotalSeconds;
        var nanos = (uint)((elapsed.Ticks % TimeSpan.TicksPerSecond) * 100);
        return Task.FromResult(new Instant { Seconds = seconds, Nanos = nanos });
    }

    public Task<TimerId> NotifyAt(NotifyAt operation)
    {
        var target = Epoch
            + TimeSpan.FromSeconds(operation.Instant.Seconds)
            + TimeSpan.FromTicks(operation.Instant.Nanos / 100);
        return Sleep(operation.Id, target - DateTime.UtcNow);
    }

    public Task<TimerId> NotifyAfter(NotifyAfter operation) =>
        // A tick is 100ns, which is the finest a `TimeSpan` goes.
        Sleep(operation.Id, TimeSpan.FromTicks((long)(operation.Duration.Nanos / 100)));

    public Task<TimerId> Clear(ClearTimer operation)
    {
        // Cancelling wakes the delay, so the task waiting on it completes and
        // answers too. Nothing acts on that answer.
        if (_timers.TryRemove(operation.Id.Value, out var cancellation))
        {
            cancellation.Cancel();
            cancellation.Dispose();
        }
        return Task.FromResult(operation.Id);
    }

    private async Task<TimerId> Sleep(TimerId id, TimeSpan delay)
    {
        var cancellation = new CancellationTokenSource();
        _timers[id.Value] = cancellation;
        try
        {
            if (delay > TimeSpan.Zero)
            {
                await Task.Delay(delay, cancellation.Token).ConfigureAwait(false);
            }
        }
        catch (OperationCanceledException)
        {
            // Cleared. Answer anyway; the core is no longer listening.
        }
        finally
        {
            if (_timers.TryRemove(id.Value, out var fired))
            {
                fired.Dispose();
            }
        }

        return id;
    }
}
