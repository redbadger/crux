import type { ClearTimer, NotifyAfter, TimerId } from "shared_types/app";

/**
 * Runs the timers of `crux_time` with `setTimeout`.
 *
 * The handler keeps a table of the timers that run, so that a `ClearTimer`
 * can cancel the timer that it names.
 */
export class TimeoutTimeHandler {
  private readonly timers = new Map<
    bigint,
    { handle: number; resolve: (id: TimerId) => void }
  >();

  /**
   * `NotifyAfter` is answered exactly once, with the id of the timer that
   * fired.
   *
   * If a `ClearTimer` for the timer arrives first, it cancels the timeout and
   * settles this promise. The core ignores that answer, because it stops
   * listening for this request when it clears the timer.
   */
  notifyAfter(operation: NotifyAfter): Promise<TimerId> {
    const millis = Number(operation.duration.nanos / BigInt(1_000_000));
    const timerId = operation.id.value;

    return new Promise((resolve) => {
      const handle = window.setTimeout(() => {
        this.timers.delete(timerId);
        resolve(operation.id);
      }, millis);
      this.timers.set(timerId, { handle, resolve });
    });
  }

  /** `ClearTimer` cancels the timer and answers with the id that it names. */
  clear(operation: ClearTimer): Promise<TimerId> {
    const timerId = operation.id.value;
    const timer = this.timers.get(timerId);
    if (timer !== undefined) {
      window.clearTimeout(timer.handle);
      this.timers.delete(timerId);
      timer.resolve(operation.id);
    }

    return Promise.resolve(operation.id);
  }
}
