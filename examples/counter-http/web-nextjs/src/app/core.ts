import type {
  EffectHandler,
  EffectSink,
  HttpRequest,
  HttpResult,
  SseRequest,
  SseResponse,
  ViewModel,
} from "shared_types/app";
import { Core, fetchHttpHandler } from "shared_types/app";

import * as sse from "./sse";

/// The shell's side of the effect protocol.
///
/// `CounterHandler` implements the generated `EffectHandler`: one method per
/// operation the app declares. The generated `EffectDispatcher` does the
/// resolving, so nothing here decides when — or how often — to call `resolve`.
///
/// There is no `render` method: the generated `Core` intercepts `Render`
/// before the dispatcher sees it and calls the `onView` callback instead.
export class CounterHandler implements EffectHandler {
  /// The handler `crux_http` ships, generated into `shared_types/app` because
  /// the codegen binary asks for it. The rules for mapping a `fetch` response
  /// belong to that crate, so the only thing this shell writes for HTTP is the
  /// line that delegates.
  private readonly httpHandler = fetchHttpHandler;

  http(operation: HttpRequest): Promise<HttpResult> {
    return this.httpHandler.request(operation);
  }

  /// `ServerSentEvents` is a stream: every item sent into `sink` is one
  /// resolution of the request that opened it.
  ///
  /// Server-Sent Events are this app's own capability (see
  /// `shared/src/sse.rs`), so no crate ships a handler for them and the shell
  /// implements the operation here — with `sse.ts`, which yields a `Chunk` per
  /// read from the response body and a final `Done`. That is the pattern for
  /// any custom capability.
  serverSentEvents(operation: SseRequest, sink: EffectSink<SseResponse>): void {
    void (async () => {
      for await (const response of sse.request(operation)) {
        sink.send(response);
      }
    })();
  }
}

/// Everything the shell has to write to run a Crux core: an `EffectHandler`
/// for the app's operations and a callback for the view model. `Core.create`
/// waits for the wasm module, builds the generated `FfiBridge` over it, and
/// owns the loop between them.
export function createCore(onView: (view: ViewModel) => void): Promise<Core> {
  return Core.create(new CounterHandler(), onView);
}
