/** The longest delay `setTimeout` honours, in milliseconds. */
const MAX_TIMEOUT = 2 ** 31 - 1;

/**
 * A `TimeHandler` whose timers are `setTimeout`s.
 *
 * The app's handler holds an instance and delegates each operation to it:
 *
 * ```ts
 * const time = new TimeoutTimeHandler();
 *
 * const handler: EffectHandler = {
 *   timeNotifyAfter: (operation) => time.notifyAfter(operation),
 *   timeClear: (operation) => time.clear(operation),
 * };
 * ```
 *
 * The timer table is state, so the app constructs one handler where it
 * constructs its own.
 *
 * ## Repeating timers
 *
 * `notifyEvery` sets a `setTimeout` for each deadline and sends a tick when it
 * fires. Each deadline is the start time plus a whole number of periods, on
 * the monotonic `performance.now()` clock, so the timer does not drift. If the
 * timeout fires after one or more deadlines have passed, the handler sends one
 * tick and waits for the next deadline that is still ahead.
 *
 * The timer runs until a `clear` for its id arrives. If the app aborts the
 * command without a `clear`, the timer continues, and the core ignores its
 * ticks.
 *
 * ## Precision
 *
 * `now` reads `Date.now()`, so it has a resolution of one millisecond.
 * `setTimeout` also counts milliseconds. A browser can clamp nested timers
 * and throttle a background tab, and a timer then fires late. The handler
 * checks the deadline when `setTimeout` returns, so a timer does not fire
 * early.
 */
export class TimeoutTimeHandler implements TimeHandler {
  /**
   * The timers that have not fired or been cleared. A one-shot timer is paired
   * with the `resolve` of the promise waiting on it. A repeating timer has no
   * promise.
   */
  private readonly timers = new Map<
    bigint,
    { handle: ReturnType<typeof setTimeout>; resolve?: (id: TimerId) => void }
  >();

  async now(_operation: Now): Promise<Instant> {
    const millis = Date.now();
    return new Instant(
      BigInt(Math.floor(millis / 1000)),
      (millis % 1000) * 1_000_000,
    );
  }

  /**
   * The wall clock gives the delay once. After that, `performance.now()`
   * measures it, because that clock does not jump when the wall clock is set.
   */
  notifyAt(operation: NotifyAt): Promise<TimerId> {
    const target =
      Number(operation.instant.seconds) * 1000 +
      operation.instant.nanos / 1_000_000;
    return this.sleep(operation.id, target - Date.now());
  }

  notifyAfter(operation: NotifyAfter): Promise<TimerId> {
    return this.sleep(operation.id, Number(operation.duration.nanos) / 1e6);
  }

  /**
   * Send `operation.id` into `sink` once per `operation.period`, until a
   * `clear` for that id arrives.
   *
   * If the app aborts the command without a `clear`, the timer continues, and
   * the core ignores its ticks.
   */
  notifyEvery(operation: NotifyEvery, sink: EffectSink<TimerId>): void {
    const id = operation.id;
    // A period of 0 would make the timer fire without a pause.
    const period = Math.max(Number(operation.period.nanos) / 1e6, 1e-6);
    const start = performance.now();
    let tick = 1;
    const timer = {
      handle: undefined as unknown as ReturnType<typeof setTimeout>,
    };
    // As in `sleep`, wait in chunks no longer than `setTimeout` allows.
    const wait = () => {
      const remaining = Math.max(start + tick * period - performance.now(), 0);
      timer.handle = setTimeout(() => {
        if (start + tick * period - performance.now() > 0) {
          wait();
          return;
        }
        // The next deadline that is still ahead, so a late timeout sends one
        // tick, not one for each period it missed.
        tick = Math.floor((performance.now() - start) / period) + 1;
        // Wait for the next tick before this one is sent. `send` can run a
        // `clear` for this timer, and that `clear` must find the new timeout.
        wait();
        sink.send(id);
      }, Math.min(remaining, MAX_TIMEOUT));
    };
    this.timers.set(id.value, timer);
    wait();
  }

  async clear(operation: ClearTimer): Promise<TimerId> {
    const timer = this.timers.get(operation.id.value);
    if (timer !== undefined) {
      clearTimeout(timer.handle);
      this.timers.delete(operation.id.value);
      // Settle the pending `notifyAt` or `notifyAfter` rather than leaving it
      // hanging. Nothing acts on that answer.
      timer.resolve?.(operation.id);
    }
    return operation.id;
  }

  private sleep(id: TimerId, millis: number): Promise<TimerId> {
    return new Promise((resolve) => {
      const deadline = performance.now() + millis;
      const timer = {
        handle: undefined as unknown as ReturnType<typeof setTimeout>,
        resolve,
      };
      // `setTimeout` treats a delay above 2^31 - 1 ms (about 24.8 days) as 0,
      // so wait in chunks no longer than that until the deadline. `clear`
      // clears whichever chunk is pending.
      const wait = () => {
        const remaining = Math.max(deadline - performance.now(), 0);
        timer.handle = setTimeout(() => {
          if (deadline - performance.now() > 0) {
            wait();
          } else {
            this.timers.delete(id.value);
            resolve(id);
          }
        }, Math.min(remaining, MAX_TIMEOUT));
      };
      this.timers.set(id.value, timer);
      wait();
    });
  }
}
