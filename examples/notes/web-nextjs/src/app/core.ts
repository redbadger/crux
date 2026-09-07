// Must come first: patches `WebAssembly.instantiate` so automerge can reach
// `crypto.getRandomValues` through boltffi's stubbed wasm-bindgen imports.
// See the module for the full explanation — importing it before `shared`
// is what guarantees the patch is installed before the WASM module loads.
import "./wasm-getrandom";

import { CoreFfi } from "shared";
import type {
  Clear,
  CoreBridge,
  EffectHandler,
  EffectSink,
  Get,
  Message,
  NotifyAfter,
  Publish,
  Set as SetValue,
  Subscribe,
  TimerId,
  ValueResult,
  ViewModel,
} from "shared_types/app";
import {
  Core,
  valueBytes,
  valueNone,
  valueResultOk,
} from "shared_types/app";
import { RefObject } from "react";

export type SyncMessage = {
  kind: "change" | "reset";
  data?: number[];
};

/// The generated `CoreBridge`, implemented over the wasm bindings: bytes in,
/// bytes out, nothing else. `CoreFfi.new()` touches the WASM module, so a
/// `LiveBridge` may only be built once `wasmInitialized` has resolved.
export class LiveBridge implements CoreBridge {
  private readonly ffi = CoreFfi.new();

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

/// The shell's side of the effect protocol.
///
/// `NotesHandler` implements the generated `EffectHandler`: one method per
/// operation the app declares, each returning the single output that operation
/// is answered with. The generated `EffectDispatcher` does the resolving —
/// nothing here calls `resolve` by hand, and there is no `render` method
/// because the generated `Core` handles `Render` itself.
export class NotesHandler implements EffectHandler {
  private readonly timers = new Map<bigint, number>();

  constructor(
    private readonly channel: RefObject<BroadcastChannel>,
    private readonly subscription: RefObject<EffectSink<Message> | null>,
  ) {}

  publish(operation: Publish): void {
    const message: SyncMessage = {
      kind: "change",
      data: operation.value,
    };
    this.channel.current.postMessage(message);
  }

  subscribe(_operation: Subscribe, sink: EffectSink<Message>): void {
    // Every message a peer broadcasts becomes one item on this sink, for as
    // long as the page lives. See `onMessage` in `page.tsx`.
    this.subscription.current = sink;
  }

  kvGet(operation: Get): Promise<ValueResult> {
    const data = window.localStorage.getItem(operation.key);
    const bytes: number[] = data == null ? [] : JSON.parse(data);

    console.log(`Loaded document (${bytes.length} bytes)`);
    return Promise.resolve(
      valueResultOk(bytes.length === 0 ? valueNone() : valueBytes(bytes)),
    );
  }

  kvSet(operation: SetValue): Promise<ValueResult> {
    console.log(`Saving document (${operation.value.length} bytes)`);
    window.localStorage.setItem(
      operation.key,
      JSON.stringify(Array.from(operation.value)),
    );
    return Promise.resolve(valueResultOk(valueNone()));
  }

  timeNotifyAfter(operation: NotifyAfter): Promise<TimerId> {
    const milliseconds = Number(operation.duration.nanos) / 1e6;
    const timerId = operation.id.value;

    return new Promise((resolve) => {
      const handle = window.setTimeout(() => {
        this.timers.delete(timerId);
        resolve(operation.id);
      }, milliseconds);
      this.timers.set(timerId, handle);
    });
  }

  timeClear(operation: Clear): void {
    const timerId = operation.id.value;
    const handle = this.timers.get(timerId);
    if (handle !== undefined) {
      window.clearTimeout(handle);
      this.timers.delete(timerId);
    }
    // The promise `timeNotifyAfter` returned is deliberately left pending: the
    // core has already given up on the timer, so resolving it now would be a
    // response to a request that no longer exists.
  }
}

/// Everything the shell has to write to run a Crux core: a `CoreBridge` over
/// the FFI, an `EffectHandler` for the app's operations, and a callback for
/// the view model. The generated `Core` owns the loop between them.
export function createCore(
  onView: (view: ViewModel) => void,
  channel: RefObject<BroadcastChannel>,
  subscription: RefObject<EffectSink<Message> | null>,
): Core {
  return new Core(
    new LiveBridge(),
    new NotesHandler(channel, subscription),
    onView,
  );
}
