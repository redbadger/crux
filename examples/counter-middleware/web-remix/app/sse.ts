import type { SseRequest } from "server_sent_events/server_sent_events";
import {
  SseResponseVariantDone,
  SseResponseVariantChunk,
} from "server_sent_events/server_sent_events";

export async function* request({ url }: SseRequest) {
  const request = new Request(url);

  const response = await fetch(request);
  if (!response.body) {
    throw new Error("SSE response has no body");
  }

  const reader = response.body.getReader();
  try {
    while (true) {
      const { done, value } = await reader.read();
      yield done
        ? new SseResponseVariantDone()
        : new SseResponseVariantChunk(Array.from(value));
      if (done) {
        break;
      }
    }
  } finally {
    reader.releaseLock();
  }
}
