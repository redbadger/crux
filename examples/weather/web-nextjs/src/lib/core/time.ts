import type { Clear, NotifyAfter, TimerId } from "shared_types/app";

/// Live timeouts, so a `Clear` can cancel the one it names — each paired with
/// the `resolve` of the `NotifyAfter` promise it belongs to.
const timers = new Map<
  bigint,
  { handle: number; resolve: (id: TimerId) => void }
>();

/// `NotifyAfter` is answered exactly once, with the id of the timer that
/// fired.
///
/// If `Clear` arrives first the timeout is cancelled and this promise settles
/// there and then, answering anyway. That is harmless: the core stops
/// listening for this request the moment it clears the timer, and ignores the
/// answer.
export function notifyAfter(operation: NotifyAfter): Promise<TimerId> {
  const millis = Number(operation.duration.nanos / BigInt(1_000_000));
  const timerId = operation.id.value;
  console.debug(`time: notify_after ${millis}ms (id=${timerId})`);

  return new Promise((resolve) => {
    const handle = window.setTimeout(() => {
      timers.delete(timerId);
      console.debug(`time: duration elapsed (id=${timerId})`);
      resolve(operation.id);
    }, millis);
    timers.set(timerId, { handle, resolve });
  });
}

/// `Clear` is a request: drop the timer and answer with the id it named.
export function clear(operation: Clear): Promise<TimerId> {
  const timerId = operation.id.value;
  console.debug(`time: clear (id=${timerId})`);
  const timer = timers.get(timerId);
  if (timer !== undefined) {
    window.clearTimeout(timer.handle);
    timers.delete(timerId);
    // Settle the `NotifyAfter` promise rather than leaving it pending. Nothing
    // acts on it — the core is no longer listening for that request.
    timer.resolve(operation.id);
  }

  return Promise.resolve(operation.id);
}
