import type {
  ClearTimer,
  NotifyAfter,
  NotifyAt,
  TimeHandler,
  TimerId,
} from "shared_types/app";
import { Instant } from "shared_types/app";

/**
 * The app's `TimeHandler`, over `window.setTimeout`.
 *
 * The handler answers each timer exactly once, with the id of the timer. If a
 * `ClearTimer` arrives first, the handler cancels the timeout and settles the
 * pending promise immediately. The core ignores that answer, because it no
 * longer listens for it.
 */
export class WindowTimeHandler implements TimeHandler {
  /**
   * The live timeouts, so that a `ClearTimer` can cancel the timeout that it
   * names. Each entry also holds the `resolve` of its pending promise.
   */
  private readonly timers = new Map<
    bigint,
    { handle: number; resolve: (id: TimerId) => void }
  >();

  async now(): Promise<Instant> {
    const millis = Date.now();
    return new Instant(
      BigInt(Math.floor(millis / 1000)),
      (millis % 1000) * 1_000_000,
    );
  }

  notifyAt(operation: NotifyAt): Promise<TimerId> {
    const { seconds, nanos } = operation.instant;
    const at = Number(seconds) * 1000 + nanos / 1_000_000;
    return this.start(operation.id, Math.max(0, at - Date.now()));
  }

  notifyAfter(operation: NotifyAfter): Promise<TimerId> {
    return this.start(
      operation.id,
      Number(operation.duration.nanos / BigInt(1_000_000)),
    );
  }

  async clear(operation: ClearTimer): Promise<TimerId> {
    const timerId = operation.id.value;
    console.debug(`time: clear (id=${timerId})`);
    const timer = this.timers.get(timerId);
    if (timer !== undefined) {
      window.clearTimeout(timer.handle);
      this.timers.delete(timerId);
      timer.resolve(operation.id);
    }
    return operation.id;
  }

  private start(id: TimerId, millis: number): Promise<TimerId> {
    const timerId = id.value;
    console.debug(`time: start ${millis}ms (id=${timerId})`);

    return new Promise((resolve) => {
      const handle = window.setTimeout(() => {
        this.timers.delete(timerId);
        console.debug(`time: elapsed (id=${timerId})`);
        resolve(id);
      }, millis);
      this.timers.set(timerId, { handle, resolve });
    });
  }
}
