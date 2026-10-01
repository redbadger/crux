import * as sharedWasm from "shared";
import { CoreFfi } from "shared";
import type {
  CoreBridge,
  EffectHandler,
  EffectSink,
  HttpRequest,
  HttpResult,
  RandomNumberRequest,
  SseRequest,
  SseResponse,
  ViewModel,
} from "shared_types/app";
import { Core, RandomNumber, fetchHttpHandler } from "shared_types/app";

import * as sse from "./sse";

/// The shell's side of the effect protocol.
///
/// `CounterHandler` implements the generated `EffectHandler`: one method per
/// operation the app declares. The generated `EffectDispatcher` does the
/// resolving for `http` and `serverSentEvents`, so nothing here decides when
/// (or how often) to call `resolve` for them.
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
  /// `shared/src/capabilities/sse.rs`), so no crate ships a handler for them
  /// and the shell implements the operation here with `sse.ts`, which
  /// yields a `Chunk` per read from the response body and a final `Done`.
  serverSentEvents(operation: SseRequest, sink: EffectSink<SseResponse>): void {
    void (async () => {
      for await (const response of sse.request(operation)) {
        sink.send(response);
      }
    })();
  }

  /// On native targets the `EffectRouter` routes `Random` to the Rust
  /// `RngHandler`, so those shells never see it. That can't run in wasm (it
  /// needs a thread), so in this shell `Random` comes through like any other
  /// effect and is answered here.
  random(operation: RandomNumberRequest): Promise<RandomNumber> {
    const min = Number(operation.field0);
    const max = Number(operation.field1);
    const result = Math.floor(Math.random() * (max - min + 1)) + min;

    return Promise.resolve(new RandomNumber(BigInt(result)));
  }
}

/// The generated `CoreBridge`, written by hand over this app's `CoreFfi`.
///
/// The codegen binary doesn't ask for the generated `FfiBridge`, because that
/// constructs `CoreFfi` with no arguments, and this app's `CoreFfi::new` takes
/// the `CruxShell` callback that the native `EffectRouter` delivers effects
/// through. So the bridge takes that callback and hands it to `CoreFfi.new`;
/// the three methods are bytes in, bytes out.
export class RoutingBridge implements CoreBridge {
  private readonly ffi: CoreFfi;

  constructor(processEffects: (bytes: Uint8Array) => void) {
    this.ffi = CoreFfi.new({ processEffects });
  }

  update(event: Uint8Array): Uint8Array {
    return this.ffi.update(event);
  }

  resolve(id: number, output: Uint8Array): Uint8Array {
    return this.ffi.resolve(id, output);
  }

  view(): Uint8Array {
    return this.ffi.view();
  }
}

const wasmInitialized = (
  sharedWasm as unknown as { initialized: Promise<void> }
).initialized;

/// Everything the shell has to write to run a Crux core with a callback: an
/// `EffectHandler`, a `CoreBridge` and a callback for the view model.
///
/// The bridge needs the `CoreFfi`, the `CoreFfi` needs the callback, and the
/// callback needs the `Core`, which is built from the bridge. The callback
/// closes over `core` and reads it when it is called, which is only ever
/// after the `Core` exists, so the cycle is broken by assigning `core` once
/// it has been constructed.
///
/// In wasm the router is compiled out (see `shared/src/ffi.rs`), so every
/// effect comes back from `update` or `resolve` and the callback is never
/// called. It's wired anyway, so that the shell is the same shape as the
/// native ones.
export async function createCore(
  onView: (view: ViewModel) => void,
): Promise<Core> {
  // `CoreFfi.new` reaches into the wasm module, so wait for it to load.
  await wasmInitialized;

  let core: Core | null = null;
  const bridge = new RoutingBridge((bytes) => core?.processBytes(bytes));
  core = new Core(bridge, new CounterHandler(), onView);
  return core;
}
