import type {
  ClearTimer,
  EffectSink,
  NotifyAfter,
  NotifyAt,
  NotifyEvery,
  TimeHandler,
  TimerId,
} from "shared_types/app";
import { Instant } from "shared_types/app";

/**
 * The app's `TimeHandler`, over `window.setTimeout`.
 *
 * The handler answers each one-shot timer exactly once, with the id of the
 * timer. If a `ClearTimer` arrives first, the handler cancels the timeout and
 * settles the pending promise immediately. The core ignores that answer,
 * because it no longer listens for it.
 *
 * A repeating timer is a `window.setInterval` that sends the id of the timer
 * into the sink once per period, until a `ClearTimer` cancels the interval.
 */
export class WindowTimeHandler implements TimeHandler {
  /**
   * The live timers, so that a `ClearTimer` can cancel the timer that it
   * names. Each entry is the function that cancels its timer. For a one-shot
   * timer, that function also settles the pending promise.
   */
  private readonly timers = new Map<bigint, () => void>();

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

  notifyEvery(operation: NotifyEvery, sink: EffectSink<TimerId>): void {
    const id = operation.id;
    const timerId = id.value;
    // A period under one millisecond becomes one millisecond.
    const millis = Math.max(
      1,
      Number(operation.period.nanos / BigInt(1_000_000)),
    );
    console.debug(`time: start every ${millis}ms (id=${timerId})`);

    const handle = window.setInterval(() => sink.send(id), millis);
    this.timers.set(timerId, () => window.clearInterval(handle));
  }

  async clear(operation: ClearTimer): Promise<TimerId> {
    const timerId = operation.id.value;
    console.debug(`time: clear (id=${timerId})`);
    const cancel = this.timers.get(timerId);
    if (cancel !== undefined) {
      this.timers.delete(timerId);
      cancel();
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
      this.timers.set(timerId, () => {
        window.clearTimeout(handle);
        resolve(id);
      });
    });
  }
}
