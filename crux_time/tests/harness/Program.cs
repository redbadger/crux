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
var cleared = await time.Clear(new Clear { Id = new TimerId { Value = 4 } });
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
var unknown = await time.Clear(new Clear { Id = new TimerId { Value = 99 } });
Expect("Clear of an unknown timer answers with the id it was given", unknown.Value, 99UL);

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
