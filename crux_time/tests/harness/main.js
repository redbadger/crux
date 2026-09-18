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
  const cleared = await time.clear(new shared.Clear(new shared.TimerId(4n)));
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
  const unknown = await time.clear(new shared.Clear(new shared.TimerId(99n)));
  expect("clear of an unknown timer answers with the id it was given", unknown.value, 99n);
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
