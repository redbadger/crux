import type { TimeRequest, TimeResponse } from "shared_types/app";
import {
  matchTimeRequest,
  timeResponseNow,
  timeResponseDurationElapsed,
  timeResponseInstantArrived,
  timeResponseCleared,
  Instant,
} from "shared_types/app";

export async function handle(request: TimeRequest): Promise<TimeResponse> {
  return matchTimeRequest<Promise<TimeResponse>>(request, {
    Now: async () => {
      console.debug("time: now");
      return timeResponseNow(nowInstant());
    },
    NotifyAfter: async (r) => {
      const millis = Number(r.duration.nanos / BigInt(1_000_000));
      console.debug(`time: notify_after ${millis}ms (id=${r.id})`);
      await sleep(millis);
      console.debug(`time: duration elapsed (id=${r.id})`);
      return timeResponseDurationElapsed(r.id);
    },
    NotifyAt: async (r) => {
      const targetMs = instantToEpochMs(r.instant);
      const nowMs = Date.now();
      console.debug(
        `time: notify_at target=${targetMs}ms now=${nowMs}ms (id=${r.id})`,
      );
      if (targetMs > nowMs) {
        await sleep(targetMs - nowMs);
      }
      console.debug(`time: instant arrived (id=${r.id})`);
      return timeResponseInstantArrived(r.id);
    },
    Clear: async (r) => {
      console.debug(`time: clear (id=${r.id})`);
      return timeResponseCleared(r.id);
    },
  });
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
