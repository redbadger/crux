/// The shell side of `crux_time`, as one method per operation.
///
/// Each method's shape is the generated `EffectHandler`'s, so an app's handler
/// holds an instance and delegates in one line:
///
/// ```ts
/// const time = new TimeoutTimeHandler();
///
/// const handler: EffectHandler = {
///   timeNotifyAfter: (operation) => time.notifyAfter(operation),
///   timeClear: (operation) => time.clear(operation),
/// };
/// ```
///
/// ## What a timer answers with, and when
///
/// `notifyAt` and `notifyAfter` are answered exactly once, with the timer's own
/// id, when it fires. `clear` cancels the timer it names and is answered with
/// the same id.
///
/// A cleared timer's `notifyAt` or `notifyAfter` may still answer — a shell
/// need not race the two. By then the core has stopped listening for that
/// answer and ignores it, which is why the implementation below settles the
/// pending promise rather than leaving it unresolved for the page's lifetime.
export interface TimeHandler {
  /// The current wall-clock time.
  now(operation: Now): Promise<Instant>;
  /// Answer with `operation.id` once `operation.instant` has arrived.
  notifyAt(operation: NotifyAt): Promise<TimerId>;
  /// Answer with `operation.id` once `operation.duration` has elapsed.
  notifyAfter(operation: NotifyAfter): Promise<TimerId>;
  /// Cancel the timer `operation.id` names, and answer with it.
  clear(operation: ClearTimer): Promise<TimerId>;
}

/// The longest delay `setTimeout` honours, in milliseconds.
const MAX_TIMEOUT = 2 ** 31 - 1;

/// A `TimeHandler` whose timers are `setTimeout`s.
///
/// The timer table is state, so the app constructs one handler where it
/// constructs its own.
export class TimeoutTimeHandler implements TimeHandler {
  /// The timers that have not fired or been cleared, each paired with the
  /// `resolve` of the promise waiting on it.
  private readonly timers = new Map<
    bigint,
    { handle: ReturnType<typeof setTimeout>; resolve: (id: TimerId) => void }
  >();

  async now(_operation: Now): Promise<Instant> {
    const millis = Date.now();
    return new Instant(
      BigInt(Math.floor(millis / 1000)),
      (millis % 1000) * 1_000_000,
    );
  }

  notifyAt(operation: NotifyAt): Promise<TimerId> {
    const target =
      Number(operation.instant.seconds) * 1000 +
      operation.instant.nanos / 1_000_000;
    return this.sleep(operation.id, target - Date.now());
  }

  notifyAfter(operation: NotifyAfter): Promise<TimerId> {
    return this.sleep(operation.id, Number(operation.duration.nanos) / 1e6);
  }

  async clear(operation: ClearTimer): Promise<TimerId> {
    const timer = this.timers.get(operation.id.value);
    if (timer !== undefined) {
      clearTimeout(timer.handle);
      this.timers.delete(operation.id.value);
      // Settle the pending `notifyAt` or `notifyAfter` rather than leaving it
      // hanging. Nothing acts on that answer.
      timer.resolve(operation.id);
    }
    return operation.id;
  }

  private sleep(id: TimerId, millis: number): Promise<TimerId> {
    return new Promise((resolve) => {
      const deadline = Date.now() + millis;
      const timer = {
        handle: undefined as unknown as ReturnType<typeof setTimeout>,
        resolve,
      };
      // `setTimeout` treats a delay above 2^31 - 1 ms (about 24.8 days) as 0,
      // so wait in chunks no longer than that until the deadline. `clear`
      // clears whichever chunk is pending.
      const wait = () => {
        const remaining = Math.max(deadline - Date.now(), 0);
        timer.handle = setTimeout(() => {
          if (deadline - Date.now() > 0) {
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
