import type { SseResponse } from "shared_types/app";
import {
  SseRequest,
  sseResponseDone,
  sseResponseChunk,
  serializeSseResponse,
} from "shared_types/app";
import type { Serializer } from "shared_types/serde";

export async function* request({ url }: SseRequest): AsyncGenerator<SseResponse> {
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
        ? sseResponseDone()
        : sseResponseChunk(Array.from(value));
      if (done) {
        break;
      }
    }
  } finally {
    reader.releaseLock();
  }
}

export function serializeResponse(response: SseResponse, serializer: Serializer): void {
  serializeSseResponse(response, serializer);
}
