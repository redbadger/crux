import type { TimeRequest, TimeResponse } from "shared_types/app";
import {
  TimeRequestVariantNow,
  TimeRequestVariantNotifyAfter,
  TimeRequestVariantNotifyAt,
  TimeRequestVariantClear,
  TimeResponseVariantNow,
  TimeResponseVariantDurationElapsed,
  TimeResponseVariantInstantArrived,
  TimeResponseVariantCleared,
  Instant,
} from "shared_types/app";

export async function handle(request: TimeRequest): Promise<TimeResponse> {
  if (request instanceof TimeRequestVariantNow) {
    console.debug("time: now");
    return new TimeResponseVariantNow(nowInstant());
  }

  if (request instanceof TimeRequestVariantNotifyAfter) {
    const millis = Number(request.duration.nanos / BigInt(1_000_000));
    console.debug(`time: notify_after ${millis}ms (id=${request.id})`);
    await sleep(millis);
    console.debug(`time: duration elapsed (id=${request.id})`);
    return new TimeResponseVariantDurationElapsed(request.id);
  }

  if (request instanceof TimeRequestVariantNotifyAt) {
    const targetMs = instantToEpochMs(request.instant);
    const nowMs = Date.now();
    console.debug(
      `time: notify_at target=${targetMs}ms now=${nowMs}ms (id=${request.id})`,
    );
    if (targetMs > nowMs) {
      await sleep(targetMs - nowMs);
    }
    console.debug(`time: instant arrived (id=${request.id})`);
    return new TimeResponseVariantInstantArrived(request.id);
  }

  if (request instanceof TimeRequestVariantClear) {
    console.debug(`time: clear (id=${request.id})`);
    return new TimeResponseVariantCleared(request.id);
  }

  throw new Error(`Unhandled time operation: ${request.constructor.name}`);
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function nowInstant(): Instant {
  const ms = Date.now();
  const seconds = BigInt(Math.floor(ms / 1000));
  const nanos = (ms % 1000) * 1_000_000;
  return new Instant(seconds, nanos);
}

function instantToEpochMs(instant: Instant): number {
  return Number(instant.seconds) * 1000 + Number(instant.nanos) / 1_000_000;
}
