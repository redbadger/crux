// Runs `TimeoutTimeHandler` through the whole `crux_time` protocol and prints
// `HARNESS OK` when every answer was the one the protocol promises.
//
// The generated module is CommonJS — `typescript()` emits `.js` beside the
// `.d.ts` it type-checks — so the harness is plain JavaScript and `node` runs
// it with no build step of its own. The types are already checked by the
// compile test above; what is left to find out is what the code does.
//
// Timing is asserted as ordering and identity, not as precision: a timer is
// answered with its own id, a cleared one settles rather than waiting out its
// duration, and a duration is not answered before it has elapsed. Every
// duration here is tens of milliseconds, except the one that is meant to be
// cleared long before it fires.

const shared = require("../generated/shared_types.js");

/// Every mismatch, so that one run reports all of them rather than the first.
const failures = [];

/// `JSON.stringify` refuses a `BigInt`, and a `TimerId` holds one.
const show = (value) => JSON.stringify(value, (_, v) => (typeof v === "bigint" ? `${v}n` : v));

const expect = (what, actual, expected) => {
  const got = show(actual);
  const wanted = show(expected);
  if (got !== wanted) failures.push(`${what}: expected ${wanted}, got ${got}`);
};

const pause = (millis) => new Promise((resolve) => setTimeout(resolve, millis));

const main = async () => {
  const time = new shared.TimeoutTimeHandler();

  // `now` answers with a wall clock: some time after this source was written,
  // and well before the century is out.
  const now = await time.now(new shared.Now());
  expect(
    "now answers with a plausible number of seconds",
    now.seconds >= 1_700_000_000n && now.seconds <= 4_102_444_800n,
    true,
  );
  expect("now answers with a fraction of a second", now.nanos < 1_000_000_000, true);

  // A timer is answered with its own id, once its duration has elapsed.
  const started = Date.now();
  const fired = await time.notifyAfter(new shared.NotifyAfter(new shared.TimerId(1n), new shared.Duration(50_000_000n)));
  const elapsed = Date.now() - started;
  expect("notifyAfter answers with the id it was given", fired.value, 1n);
  expect(`notifyAfter waits for the duration it was given, ${elapsed}ms`, elapsed >= 30, true);

  const later = await time.now(new shared.Now());
  expect(
    "now does not go backwards",
    later.seconds > now.seconds || (later.seconds === now.seconds && later.nanos >= now.nanos),
    true,
  );

  // Two at once, each answered with its own id rather than with the other's.
  const [two, three] = await Promise.all([
    time.notifyAfter(new shared.NotifyAfter(new shared.TimerId(2n), new shared.Duration(40_000_000n))),
    time.notifyAfter(new shared.NotifyAfter(new shared.TimerId(3n), new shared.Duration(10_000_000n))),
  ]);
  expect("timers running at once keep their own ids", [two.value, three.value], [2n, 3n]);

  // A timer that is cleared before it fires. Its `notifyAfter` still settles —
  // the core has stopped listening by then, so what it answers with does not
  // matter, but a promise that never settles is one the page waits on for
  // good.
  const pending = time.notifyAfter(
    new shared.NotifyAfter(new shared.TimerId(4n), new shared.Duration(10_000_000_000n)),
  );
  // Long enough for the handler to have the timer in its table before it is
  // asked to take it out again.
  await pause(50);
  const cleared = await time.clear(new shared.ClearTimer(new shared.TimerId(4n)));
  expect("clear answers with the id it was given", cleared.value, 4n);

  const waited = Date.now();
  await pending;
  const settling = Date.now() - waited;
  expect(
    `a cleared timer's notifyAfter settles rather than waiting out its duration, ${settling}ms`,
    settling < 5_000,
    true,
  );

  // Clearing a timer that is not there is not an error: the shell need not
  // race a timer that has already fired.
  const unknown = await time.clear(new shared.ClearTimer(new shared.TimerId(99n)));
  expect("clear of an unknown timer answers with the id it was given", unknown.value, 99n);

  // A timer past `setTimeout`'s limit of 2^31 - 1 ms (about 24.8 days), which
  // treats a longer delay as 0 and would fire it at once. The clock and the
  // timers are stubbed so that no real time is spent: a stubbed timer only
  // fires when the harness fires it.
  const realNow = Date.now;
  const realSetTimeout = globalThis.setTimeout;
  const realClearTimeout = globalThis.clearTimeout;
  const limit = 2 ** 31 - 1;
  let offset = 0;
  let scheduled = [];
  Date.now = () => realNow() + offset;
  globalThis.setTimeout = (callback, delay) => {
    const timer = { callback, delay, cleared: false };
    scheduled.push(timer);
    return timer;
  };
  globalThis.clearTimeout = (timer) => {
    timer.cleared = true;
  };
  // Let the clock reach the pending timer, then run it.
  const fireNext = (what) => {
    const timer = scheduled.find((t) => !t.cleared);
    expect(what, timer !== undefined && timer.delay <= limit, true);
    if (timer === undefined) return;
    scheduled = scheduled.filter((t) => t !== timer);
    offset += timer.delay;
    timer.callback();
  };
  try {
    const thirtyDays = 30n * 24n * 3600n * 1_000_000_000n;
    let answered;
    time
      .notifyAfter(new shared.NotifyAfter(new shared.TimerId(5n), new shared.Duration(thirtyDays)))
      .then((id) => (answered = id));
    expect("a long timer waits in chunks no longer than setTimeout allows", scheduled.map((t) => t.delay <= limit), [true]);
    fireNext("a 30-day timer schedules its first wait within setTimeout's limit");
    await Promise.resolve();
    expect("a long timer does not answer after its first chunk", answered, undefined);
    expect("a long timer waits again for the rest", scheduled.map((t) => t.delay > 0 && t.delay <= limit), [true]);
    fireNext("a long timer re-arms after its first chunk");
    await Promise.resolve();
    expect("a long timer answers with its id once the deadline arrives", answered?.value, 5n);

    // Cleared part-way through, the timer clears the chunk that is pending.
    let cancelled;
    const far = time.notifyAfter(new shared.NotifyAfter(new shared.TimerId(6n), new shared.Duration(thirtyDays)));
    far.then((id) => (cancelled = id));
    fireNext("a second 30-day timer schedules its first wait within setTimeout's limit");
    const chunk = scheduled.find((t) => !t.cleared);
    await time.clear(new shared.ClearTimer(new shared.TimerId(6n)));
    expect("clear cancels the chunk that is pending", chunk?.cleared, true);
    await far;
    expect("a cleared long timer settles with its id", cancelled?.value, 6n);
  } finally {
    Date.now = realNow;
    globalThis.setTimeout = realSetTimeout;
    globalThis.clearTimeout = realClearTimeout;
  }
};

// A hung answer would otherwise leave node with nothing to do and no reason to
// say so: an unsettled promise is not an error, and the process exits zero
// without ever reaching the end of `main`. The watchdog is deliberately not
// `unref`ed, so that it is the thing keeping the loop alive.
const watchdog = setTimeout(() => {
  console.error("timed out: an operation never answered");
  process.exit(2);
}, 60_000);

main().then(() => {
  clearTimeout(watchdog);
  if (failures.length === 0) {
    console.log("HARNESS OK");
  } else {
    for (const failure of failures) console.log(`FAILED: ${failure}`);
    process.exitCode = 1;
  }
});
