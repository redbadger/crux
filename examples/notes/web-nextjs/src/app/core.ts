// Must come first: patches `WebAssembly.instantiate` so automerge can reach
// `crypto.getRandomValues` through boltffi's stubbed wasm-bindgen imports.
// See the module for the full explanation — importing it before anything that
// evaluates `shared` (which now happens through `shared_types/app`, because
// the generated `FfiBridge` calls into it) is what guarantees the patch is
// installed before the WASM module loads.
import "./wasm-getrandom";

import type {
  ClearTimer,
  EffectHandler,
  EffectSink,
  GetValue,
  Message,
  NotifyAfter,
  Publish,
  SetValue,
  Subscribe,
  TimerId,
  ValueResult,
  ViewModel,
} from "shared_types/app";
import {
  Core,
  createLocalStorageKeyValueHandler,
  TimeoutTimeHandler,
} from "shared_types/app";
import { RefObject } from "react";

export type SyncMessage = {
  kind: "change" | "reset";
  data?: number[];
};

/// The shell's side of the effect protocol.
///
/// `NotesHandler` implements the generated `EffectHandler`: one method per
/// operation the app declares, each returning the single output that operation
/// is answered with. The generated `EffectDispatcher` does the resolving —
/// nothing here calls `resolve` by hand, and there is no `render` method
/// because the generated `Core` handles `Render` itself.
///
/// The store and the timers are `crux_kv`'s and `crux_time`'s own business, so
/// this shell uses the handlers those crates ship — generated into
/// `shared_types/app` because the codegen binary asks for them — and writes only
/// the line that delegates. Publishing and subscribing are this app's, and are
/// written out below.
export class NotesHandler implements EffectHandler {
  /// No prefix, so the documents this app has already saved under their own
  /// keys are still where it left them.
  private readonly kv = createLocalStorageKeyValueHandler();
  /// The timer table is state, so there is one handler, for the page's life.
  private readonly time = new TimeoutTimeHandler();

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

  kvGet(operation: GetValue): Promise<ValueResult> {
    return this.kv.get(operation);
  }

  kvSet(operation: SetValue): Promise<ValueResult> {
    return this.kv.set(operation);
  }

  timeNotifyAfter(operation: NotifyAfter): Promise<TimerId> {
    return this.time.notifyAfter(operation);
  }

  timeClear(operation: ClearTimer): Promise<TimerId> {
    return this.time.clear(operation);
  }
}

/// Everything the shell has to write to run a Crux core: an `EffectHandler`
/// for the app's operations and a callback for the view model. `Core.create`
/// waits for the wasm module, builds the generated `FfiBridge` over it, and
/// owns the loop between them.
export function createCore(
  onView: (view: ViewModel) => void,
  channel: RefObject<BroadcastChannel>,
  subscription: RefObject<EffectSink<Message> | null>,
): Promise<Core> {
  return Core.create(new NotesHandler(channel, subscription), onView);
}
