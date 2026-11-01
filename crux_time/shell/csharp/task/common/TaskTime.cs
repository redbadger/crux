using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading;
using System.Threading.Tasks;

/// <summary>
/// An <see cref="ITimeHandler"/> whose timers are <c>Task.Delay</c>s.
/// </summary>
/// <remarks>
/// <para>
/// The app's handler holds an instance and delegates each operation to it:
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
/// The timer table is state, so the app constructs one handler where it
/// constructs its own.
/// </para>
/// <para>
/// <b>Precision.</b> <c>Now</c> reads <c>DateTime.UtcNow</c>, which counts
/// 100 ns ticks. Its real resolution depends on the operating system. A timer
/// waits a whole number of milliseconds, rounded up, and the operating system
/// timer can be coarser: about 15 ms on Windows. A timer can fire late. It
/// can also fire early by under 100 ns, because the conversion from
/// nanoseconds to ticks truncates.
/// </para>
/// </remarks>
public sealed class TaskTimeHandler : ITimeHandler
{
    private static readonly DateTime Epoch = new(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc);

    /// <summary>The longest single delay <c>Task.Delay</c> accepts.</summary>
    private static readonly TimeSpan MaxDelay = TimeSpan.FromMilliseconds(uint.MaxValue - 1);

    /// <summary>The timers that have not fired or been cleared, by id.</summary>
    private readonly ConcurrentDictionary<ulong, CancellationTokenSource> _timers = new();

    /// <summary>The largest whole number of seconds that fits in a <c>TimeSpan</c>, less one.</summary>
    private const ulong MaxSeconds = (ulong)(long.MaxValue / TimeSpan.TicksPerSecond) - 1;

    public Task<Instant> Now(Now operation) => Task.FromResult(FromElapsed(DateTime.UtcNow - Epoch));

    /// <summary>
    /// The <c>Instant</c> that is <paramref name="elapsed"/> after the Unix
    /// epoch, or the epoch itself if <paramref name="elapsed"/> is negative.
    /// </summary>
    /// <remarks>
    /// The calculation uses whole ticks, not <c>TotalSeconds</c>. A
    /// <c>double</c> can round up to the next second, which would put the
    /// answer one second ahead.
    /// </remarks>
    public static Instant FromElapsed(TimeSpan elapsed)
    {
        var ticks = Math.Max(elapsed.Ticks, 0);
        return new Instant
        {
            Seconds = (ulong)(ticks / TimeSpan.TicksPerSecond),
            Nanos = (uint)(ticks % TimeSpan.TicksPerSecond * 100),
        };
    }

    public Task<TimerId> NotifyAt(NotifyAt operation)
    {
        // The wall clock gives the delay once. After that, the delay is
        // measured on a clock that does not jump when the wall clock is set.
        // An instant too far ahead for a `TimeSpan` waits for as long as one
        // can hold.
        var target = (long)Math.Min(operation.Instant.Seconds, MaxSeconds) * TimeSpan.TicksPerSecond
            + operation.Instant.Nanos / 100;
        return Sleep(operation.Id, TimeSpan.FromTicks(target - (DateTime.UtcNow - Epoch).Ticks));
    }

    public Task<TimerId> NotifyAfter(NotifyAfter operation) =>
        // A tick is 100ns, which is the finest a `TimeSpan` goes.
        Sleep(operation.Id, TimeSpan.FromTicks((long)(operation.Duration.Nanos / 100)));

    public Task<TimerId> Clear(ClearTimer operation)
    {
        // Cancelling wakes the delay, so the task waiting on it completes and
        // answers too. Nothing acts on that answer. The sleeping task takes the
        // timer out of the table and disposes of it.
        if (_timers.TryGetValue(operation.Id.Value, out var cancellation))
        {
            try
            {
                cancellation.Cancel();
            }
            catch (ObjectDisposedException)
            {
                // The timer fired and was disposed of after the lookup.
            }
        }
        return Task.FromResult(operation.Id);
    }

    private async Task<TimerId> Sleep(TimerId id, TimeSpan delay)
    {
        var cancellation = new CancellationTokenSource();
        var token = cancellation.Token;
        _timers[id.Value] = cancellation;
        try
        {
            // `Stopwatch` measures the delay, because it does not jump when the
            // wall clock is set. `Task.Delay` throws for a delay above
            // `uint.MaxValue - 1` ms (about 49.7 days), so wait in chunks no
            // longer than that. It also truncates to whole milliseconds, so
            // round up: a timer must not fire early, and a delay of 0 ms would
            // make this loop spin.
            var start = Stopwatch.GetTimestamp();
            while (delay - Stopwatch.GetElapsedTime(start) is var remaining && remaining > TimeSpan.Zero)
            {
                var chunk = remaining < MaxDelay
                    ? TimeSpan.FromMilliseconds(Math.Ceiling(remaining.TotalMilliseconds))
                    : MaxDelay;
                await Task.Delay(chunk, token).ConfigureAwait(false);
            }
        }
        catch (OperationCanceledException)
        {
            // Cleared. Answer anyway; the core is no longer listening.
        }
        finally
        {
            // Only this timer's own entry: a later timer can have the same id.
            _timers.TryRemove(new KeyValuePair<ulong, CancellationTokenSource>(id.Value, cancellation));
            cancellation.Dispose();
        }

        return id;
    }
}
