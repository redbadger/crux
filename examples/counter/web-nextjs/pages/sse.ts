import {
  SseRequest,
  SseResponseVariantDone,
  SseResponseVariantChunk,
} from "shared_types/types/shared_types";

export async function* request({ url }: SseRequest) {
  const request = new Request(url);

  const response = await fetch(request);

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
