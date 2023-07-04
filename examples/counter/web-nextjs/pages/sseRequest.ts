import {
  SseRequest,
  SseResponseVariantDone,
  SseResponseVariantChunk,
} from "shared_types/types/shared_types";

export async function* sseRequest(sseRequest: SseRequest) {
  const req = new Request(sseRequest.url);

  const res = await fetch(req);

  const reader = res.body.getReader();
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
