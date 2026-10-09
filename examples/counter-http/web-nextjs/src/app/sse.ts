import type { SseRequest } from "shared_types/app";
import { sseResponseDone, sseResponseChunk } from "shared_types/app";

export async function* request({ url }: SseRequest) {
  const request = new Request(url);

  const response = await fetch(request);
  if (!response.ok || !response.body) {
    throw new Error(`SSE request failed: ${response.status}`);
  }

  const reader = response.body.getReader();
  try {
    while (true) {
      const { done, value } = await reader.read();
      yield done ? sseResponseDone() : sseResponseChunk(Array.from(value));
      if (done) {
        break;
      }
    }
  } finally {
    reader.releaseLock();
  }
}
