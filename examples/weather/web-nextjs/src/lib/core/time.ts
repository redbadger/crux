import type { Clear, NotifyAfter, TimerId } from "shared_types/app";

/// Live timeouts, so a `Clear` can cancel the one it names.
const timers = new Map<bigint, number>();

/// `NotifyAfter` is answered exactly once, with the id of the timer that
/// fired.
///
/// If `Clear` arrives first this promise is deliberately left pending:
/// `Clear` is a notification, so the core has already stopped waiting for the
/// timer and would reject a late answer.
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
    timers.set(timerId, handle);
  });
}

/// `Clear` is a notification: drop the timer and answer nothing.
export function clear(operation: Clear): void {
  const timerId = operation.id.value;
  console.debug(`time: clear (id=${timerId})`);
  const handle = timers.get(timerId);
  if (handle !== undefined) {
    window.clearTimeout(handle);
    timers.delete(timerId);
  }
}
