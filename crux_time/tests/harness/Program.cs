// Runs `TaskTimeHandler` through the whole `crux_time` protocol and prints
// `HARNESS OK` when every answer was the one the protocol promises.
//
// Timing is asserted as ordering and identity, not as precision: a timer is
// answered with its own id, a cleared one settles rather than waiting out its
// duration, and a duration is not answered before it has elapsed. Every
// duration here is tens of milliseconds, except the one that is meant to be
// cleared long before it fires.

using System.Diagnostics;
using Example.Shared;

// A hung answer would otherwise wait for the test's own timeout, with nothing
// to say for itself.
_ = Task.Run(async () =>
{
    await Task.Delay(TimeSpan.FromSeconds(60));
    Console.Error.WriteLine("timed out: an operation never answered");
    Environment.Exit(2);
});

// Every mismatch, so that one run reports all of them rather than the first.
var failures = new List<string>();

void Expect<T>(string what, T actual, T expected)
{
    if (!Show(actual).Equals(Show(expected), StringComparison.Ordinal))
    {
        failures.Add($"{what}: expected {Show(expected)}, got {Show(actual)}");
    }
}

static string Show<T>(T value) =>
    value switch
    {
        null => "null",
        IEnumerable<ulong> ids => "[" + string.Join(",", ids) + "]",
        _ => value.ToString() ?? "null",
    };

ITimeHandler time = new TaskTimeHandler();

// `Now` answers with a wall clock: some time after this source was written,
// and well before the century is out.
var now = await time.Now(new Now());
Expect("Now answers with a plausible number of seconds", now.Seconds is >= 1_700_000_000 and <= 4_102_444_800, true);
Expect("Now answers with a fraction of a second", now.Nanos < 1_000_000_000, true);

// A timer is answered with its own id, once its duration has elapsed.
var started = Stopwatch.StartNew();
var fired = await time.NotifyAfter(
    new NotifyAfter { Id = new TimerId { Value = 1 }, Duration = new Duration { Nanos = 50_000_000 } }
);
var elapsed = started.ElapsedMilliseconds;
Expect("NotifyAfter answers with the id it was given", fired.Value, 1UL);
Expect($"NotifyAfter waits for the duration it was given, {elapsed}ms", elapsed >= 30, true);

var later = await time.Now(new Now());
Expect(
    "Now does not go backwards",
    later.Seconds > now.Seconds || (later.Seconds == now.Seconds && later.Nanos >= now.Nanos),
    true
);

// Two at once, each answered with its own id rather than with the other's.
var two = time.NotifyAfter(
    new NotifyAfter { Id = new TimerId { Value = 2 }, Duration = new Duration { Nanos = 40_000_000 } }
);
var three = time.NotifyAfter(
    new NotifyAfter { Id = new TimerId { Value = 3 }, Duration = new Duration { Nanos = 10_000_000 } }
);
var both = await Task.WhenAll(two, three);
Expect("timers running at once keep their own ids", both.Select(id => id.Value).ToList(), [2UL, 3UL]);

// A timer that is cleared before it fires. Its `NotifyAfter` still settles —
// the core has stopped listening by then, so what it answers with does not
// matter, but a task that never completes is one the app awaits for good.
var pending = time.NotifyAfter(
    new NotifyAfter { Id = new TimerId { Value = 4 }, Duration = new Duration { Nanos = 10_000_000_000 } }
);
// Long enough for the handler to have the timer in its table before it is
// asked to take it out again.
await Task.Delay(50);
var cleared = await time.Clear(new ClearTimer { Id = new TimerId { Value = 4 } });
Expect("Clear answers with the id it was given", cleared.Value, 4UL);

var waited = Stopwatch.StartNew();
await pending;
var settling = waited.ElapsedMilliseconds;
Expect(
    $"a cleared timer's NotifyAfter settles rather than waiting out its duration, {settling}ms",
    settling < 5_000,
    true
);

// Clearing a timer that is not there is not an error: the shell need not race
// a timer that has already fired.
var unknown = await time.Clear(new ClearTimer { Id = new TimerId { Value = 99 } });
Expect("Clear of an unknown timer answers with the id it was given", unknown.Value, 99UL);

// A timer past `Task.Delay`'s limit of `uint.MaxValue - 1` ms (about 49.7
// days), which throws. It must stay pending rather than fault, and clearing it
// must still answer. No real time is spent: the timer is cleared at once.
var sixtyDays = 60UL * 24 * 3600 * 1_000_000_000;
var far = time.NotifyAfter(
    new NotifyAfter { Id = new TimerId { Value = 5 }, Duration = new Duration { Nanos = sixtyDays } }
);
await Task.Delay(50);
Expect("a timer beyond Task.Delay's limit stays pending", far.IsCompleted, false);
var farCleared = await time.Clear(new ClearTimer { Id = new TimerId { Value = 5 } });
Expect("Clear of a timer beyond Task.Delay's limit answers with its id", farCleared.Value, 5UL);
var farAnswer = await far;
Expect("a cleared timer beyond Task.Delay's limit settles with its id", farAnswer.Value, 5UL);

// `FromElapsed` uses whole ticks. A `double` of the total seconds rounds the
// last tick before a whole second up to that second.
var almost = TaskTimeHandler.FromElapsed(TimeSpan.FromTicks(1_700_000_000L * TimeSpan.TicksPerSecond - 1));
Expect("FromElapsed does not round up to the next second", almost.Seconds, 1_699_999_999UL);
Expect("FromElapsed keeps the last tick of the second", almost.Nanos, 999_999_900U);

// A repeating timer sends its own id once per period. The `n`th tick comes no
// earlier than `n` periods after the start, so no tick is early and the ticks
// never come in a burst.
var repeating = new Ticks();
var period = TimeSpan.FromMilliseconds(20);
var repeatingStart = Stopwatch.GetTimestamp();
new Delegating(time).TimeNotifyEvery(
    new NotifyEvery { Id = new TimerId { Value = 10 }, Period = new Duration { Nanos = 20_000_000 } },
    repeating
);
await Task.Delay(130);
var sent = repeating.All;
Expect($"NotifyEvery sends several ticks, got {sent.Count}", sent.Count >= 3, true);
Expect("NotifyEvery sends its own id each time", sent.Select(t => t.Id).Distinct().ToList(), [10UL]);
Expect(
    "NotifyEvery does not send a tick before its deadline",
    sent.Select((tick, n) => Stopwatch.GetElapsedTime(repeatingStart, tick.At) >= period * (n + 1)).All(early => early),
    true
);

// `Clear` answers with the id, and the timer stops. A tick that started before
// the `Clear` can still arrive, so one more tick is allowed, and then the count
// must stay the same.
var clearedRepeating = await time.Clear(new ClearTimer { Id = new TimerId { Value = 10 } });
Expect("Clear of a repeating timer answers with the id it was given", clearedRepeating.Value, 10UL);
var countAtClear = repeating.All.Count;
await Task.Delay(30);
var countAfterClear = repeating.All.Count;
Expect("a cleared repeating timer sends at most one more tick", countAfterClear <= countAtClear + 1, true);
await Task.Delay(70);
Expect("a cleared repeating timer sends no more ticks", repeating.All.Count, countAfterClear);

// A one-shot timer and a repeating timer side by side, each with its own id.
var beside = new Ticks();
time.NotifyEvery(new NotifyEvery { Id = new TimerId { Value = 11 }, Period = new Duration { Nanos = 10_000_000 } }, beside);
var oneShot = await time.NotifyAfter(
    new NotifyAfter { Id = new TimerId { Value = 12 }, Duration = new Duration { Nanos = 50_000_000 } }
);
Expect("a one-shot timer beside a repeating one answers with its own id", oneShot.Value, 12UL);
var besideTicks = beside.All;
Expect($"a repeating timer beside a one-shot one ticks, got {besideTicks.Count}", besideTicks.Count >= 2, true);
Expect("a repeating timer beside a one-shot one sends its own id", besideTicks.Select(t => t.Id).Distinct().ToList(), [11UL]);
await time.Clear(new ClearTimer { Id = new TimerId { Value = 11 } });
await Task.Delay(20);
var besideAtClear = beside.All.Count;
await Task.Delay(40);
Expect("the repeating timer beside a one-shot one stops when it is cleared", beside.All.Count, besideAtClear);

// A `Clear` that the sink runs, as the generated dispatcher does when an
// update on a tick clears the timer, stops the timer before its next tick.
var reentrant = new ClearingTicks(time);
time.NotifyEvery(new NotifyEvery { Id = new TimerId { Value = 13 }, Period = new Duration { Nanos = 10_000_000 } }, reentrant);
await Task.Delay(60);
Expect("a Clear inside Send stops the repeating timer", reentrant.All.Select(t => t.Id).ToList(), [13UL]);

if (failures.Count == 0)
{
    Console.WriteLine("HARNESS OK");
    return 0;
}

foreach (var failure in failures)
{
    Console.WriteLine($"FAILED: {failure}");
}
return 1;

/// <summary>
/// An app's handler, which delegates each operation in one line. It compiles
/// only if each method of <c>ITimeHandler</c> has the shape of the method of
/// the generated <c>IEffectHandler</c>.
/// </summary>
sealed class Delegating(ITimeHandler time) : IEffectHandler
{
    public Task<Instant> TimeNow(Now operation) => time.Now(operation);
    public Task<TimerId> TimeNotifyAt(NotifyAt operation) => time.NotifyAt(operation);
    public Task<TimerId> TimeNotifyAfter(NotifyAfter operation) => time.NotifyAfter(operation);
    public void TimeNotifyEvery(NotifyEvery operation, IEffectSink<TimerId> sink) => time.NotifyEvery(operation, sink);
    public Task<TimerId> TimeClear(ClearTimer operation) => time.Clear(operation);
}

/// <summary>
/// The ticks of a repeating timer: the id each one sent, and when, as a
/// <c>Stopwatch</c> timestamp.
/// </summary>
sealed class Ticks : IEffectSink<TimerId>
{
    private readonly List<(ulong Id, long At)> _ticks = [];

    public void Send(TimerId item)
    {
        lock (_ticks)
        {
            _ticks.Add((item.Value, Stopwatch.GetTimestamp()));
        }
    }

    public List<(ulong Id, long At)> All
    {
        get
        {
            lock (_ticks)
            {
                return [.. _ticks];
            }
        }
    }
}

/// <summary>
/// A sink that records each tick and then clears its timer inside
/// <c>Send</c>, on the thread of the timer.
/// </summary>
sealed class ClearingTicks(ITimeHandler time) : IEffectSink<TimerId>
{
    private readonly Ticks _ticks = new();

    public void Send(TimerId item)
    {
        _ticks.Send(item);
        _ = time.Clear(new ClearTimer { Id = item });
    }

    public List<(ulong Id, long At)> All => _ticks.All;
}
