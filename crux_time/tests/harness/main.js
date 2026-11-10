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

/** Every mismatch, so that one run reports all of them rather than the first. */
const failures = [];

/** `JSON.stringify` refuses a `BigInt`, and a `TimerId` holds one. */
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

  // A repeating timer sends its own id once per period. The `n`th tick comes no
  // earlier than `n` periods after the start, so no tick is early.
  const ticks = (sent) => ({ send: (id) => sent.push({ id: id.value, at: performance.now() }) });
  const repeating = [];
  const period = 20;
  const repeatingStart = performance.now();
  time.notifyEvery(new shared.NotifyEvery(new shared.TimerId(10n), new shared.Duration(20_000_000n)), ticks(repeating));
  await pause(130);
  expect(`notifyEvery sends several ticks, got ${repeating.length}`, repeating.length >= 3, true);
  expect("notifyEvery sends its own id each time", [...new Set(repeating.map((t) => t.id))], [10n]);
  expect(
    "notifyEvery does not send a tick before its deadline",
    repeating.every((tick, n) => tick.at - repeatingStart >= (n + 1) * period),
    true,
  );

  // `clear` answers with the id, and no tick follows the answer.
  const clearedRepeating = await time.clear(new shared.ClearTimer(new shared.TimerId(10n)));
  expect("clear of a repeating timer answers with the id it was given", clearedRepeating.value, 10n);
  const countAtClear = repeating.length;
  await pause(70);
  expect("a cleared repeating timer sends no more ticks", repeating.length, countAtClear);

  // A one-shot timer and a repeating timer side by side, each with its own id.
  const beside = [];
  time.notifyEvery(new shared.NotifyEvery(new shared.TimerId(11n), new shared.Duration(10_000_000n)), ticks(beside));
  const oneShot = await time.notifyAfter(
    new shared.NotifyAfter(new shared.TimerId(12n), new shared.Duration(50_000_000n)),
  );
  expect("a one-shot timer beside a repeating one answers with its own id", oneShot.value, 12n);
  expect(`a repeating timer beside a one-shot one ticks, got ${beside.length}`, beside.length >= 2, true);
  expect("a repeating timer beside a one-shot one sends its own id", [...new Set(beside.map((t) => t.id))], [11n]);
  await time.clear(new shared.ClearTimer(new shared.TimerId(11n)));
  const besideAtClear = beside.length;
  await pause(40);
  expect("the repeating timer beside a one-shot one stops when it is cleared", beside.length, besideAtClear);

  // A `clear` that the sink runs, as the generated dispatcher does, stops the
  // timer before its next tick.
  const reentrant = [];
  time.notifyEvery(new shared.NotifyEvery(new shared.TimerId(13n), new shared.Duration(10_000_000n)), {
    send: (id) => {
      reentrant.push(id.value);
      void time.clear(new shared.ClearTimer(id));
    },
  });
  await pause(60);
  expect("a clear inside send stops the repeating timer", reentrant, [13n]);

  // A timer past `setTimeout`'s limit of 2^31 - 1 ms (about 24.8 days), which
  // treats a longer delay as 0 and would fire it at once. The clocks and the
  // timers are stubbed so that no real time is spent: a stubbed timer only
  // fires when the harness fires it. The handler measures a delay with
  // `performance.now()`, and `Date.now()` moves with it.
  const realNow = Date.now;
  const realPerformanceNow = performance.now;
  const realSetTimeout = globalThis.setTimeout;
  const realClearTimeout = globalThis.clearTimeout;
  const limit = 2 ** 31 - 1;
  let offset = 0;
  // A change to the wall clock alone, as when the user sets the time.
  let jump = 0;
  let scheduled = [];
  Date.now = () => realNow() + offset + jump;
  performance.now = () => realPerformanceNow.call(performance) + offset;
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

    // The wall clock goes back an hour while a one-second timer waits. The
    // timer answers after one second, not after one hour and one second.
    let unmoved;
    time
      .notifyAfter(new shared.NotifyAfter(new shared.TimerId(7n), new shared.Duration(1_000_000_000n)))
      .then((id) => (unmoved = id));
    jump = -3_600_000;
    fireNext("a one-second timer schedules its wait");
    await Promise.resolve();
    expect("a timer is not extended when the wall clock goes back", unmoved?.value, 7n);
    jump = 0;

    // A repeating timer that wakes late sends one tick, not one for each
    // period that it missed, and then waits for the next deadline. The real
    // clock still moves a little under the stub, so delays are rounded.
    scheduled = [];
    const late = [];
    time.notifyEvery(new shared.NotifyEvery(new shared.TimerId(20n), new shared.Duration(100_000_000n)), {
      send: (id) => late.push(id.value),
    });
    expect("a repeating timer waits one period for its first tick", scheduled.map((t) => Math.round(t.delay)), [100]);
    // The timeout fires after three and a half periods.
    offset += 250;
    fireNext("a repeating timer schedules its first tick");
    expect("a late repeating timer sends one tick, not a burst", late, [20n]);
    expect(
      "a late repeating timer waits for the next deadline that is still ahead",
      scheduled.filter((t) => !t.cleared).map((t) => Math.round(t.delay)),
      [50],
    );
    fireNext("a late repeating timer schedules its next tick");
    expect("a late repeating timer then ticks on its deadline", late, [20n, 20n]);
    await time.clear(new shared.ClearTimer(new shared.TimerId(20n)));
    expect("clear cancels the pending tick of a repeating timer", scheduled.every((t) => t.cleared), true);

    // A repeating timer with a period past `setTimeout`'s limit waits in
    // chunks, and does not tick before its deadline.
    const slow = [];
    time.notifyEvery(new shared.NotifyEvery(new shared.TimerId(21n), new shared.Duration(thirtyDays)), {
      send: (id) => slow.push(id.value),
    });
    fireNext("a 30-day repeating timer schedules its first wait within setTimeout's limit");
    expect("a 30-day repeating timer does not tick after its first chunk", slow, []);
    fireNext("a 30-day repeating timer re-arms after its first chunk");
    expect("a 30-day repeating timer ticks once its period has elapsed", slow, [21n]);
    expect(
      "a 30-day repeating timer waits for its next period within setTimeout's limit",
      scheduled.filter((t) => !t.cleared).map((t) => t.delay <= limit),
      [true],
    );
    await time.clear(new shared.ClearTimer(new shared.TimerId(21n)));
    scheduled = [];
  } finally {
    Date.now = realNow;
    performance.now = realPerformanceNow;
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
