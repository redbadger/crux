// Must come first: patches `WebAssembly.instantiate` so automerge can reach
// `crypto.getRandomValues` through boltffi's stubbed wasm-bindgen imports.
// See the module for the full explanation — importing it before `shared`
// is what guarantees the patch is installed before the WASM module loads.
import "./wasm-getrandom";

import { CoreFfi } from "shared";
import type {
  Clear,
  EffectHandler,
  EffectSink,
  Event,
  Get,
  Message,
  NotifyAfter,
  Publish,
  Set as SetValue,
  Subscribe,
  TimerId,
  ValueResult,
} from "shared_types/app";
import {
  EffectDispatcher,
  Request,
  ViewModel,
  serializeEvent,
  valueBytes,
  valueNone,
  valueResultOk,
} from "shared_types/app";
import { BincodeSerializer, BincodeDeserializer } from "shared_types/bincode";
import { Dispatch, RefObject, SetStateAction } from "react";

export type SyncMessage = {
  kind: "change" | "reset";
  data?: number[];
};

/// The shell's side of the effect protocol.
///
/// `Core` implements the generated `EffectHandler`: one method per operation
/// the app declares, each returning the single output that operation is
/// answered with. The generated `EffectDispatcher` does the resolving —
/// nothing here calls `resolve` by hand.
export class Core implements EffectHandler {
  private core: CoreFfi | null = null;
  private readonly dispatcher: EffectDispatcher;
  /// Live timeouts, so a `Clear` can cancel the one it names — each paired
  /// with the `resolve` of the `NotifyAfter` promise it belongs to.
  private readonly timers = new Map<
    bigint,
    { handle: number; resolve: (id: TimerId) => void }
  >();

  setState: Dispatch<SetStateAction<ViewModel>>;
  channel: RefObject<BroadcastChannel>;
  subscription: RefObject<EffectSink<Message> | null>;

  constructor(
    setState: Dispatch<SetStateAction<ViewModel>>,
    channel: RefObject<BroadcastChannel>,
    subscription: RefObject<EffectSink<Message> | null>,
  ) {
    // Don't initialize CoreFfi here - wait for WASM to be loaded
    this.setState = setState;
    this.channel = channel;
    this.subscription = subscription;
    this.dispatcher = new EffectDispatcher(this, (id, bytes) => {
      this.process(this.ffi().resolve(id, bytes));
    });
  }

  initialize() {
    this.core ??= CoreFfi.new();
  }

  view(): ViewModel {
    return deserializeView(this.ffi().view());
  }

  update(event: Event) {
    console.log("event", event);

    const serializer = new BincodeSerializer();
    serializeEvent(event, serializer);

    this.process(this.ffi().update(serializer.getBytes()));
  }

  private process(effects: Uint8Array | number[]) {
    for (const request of deserializeRequests(effects)) {
      console.log("effect", request.effect);
      this.dispatcher.dispatch(request);
    }
  }

  private ffi(): CoreFfi {
    if (!this.core) {
      throw new Error("Core not initialized. Call initialize() first.");
    }
    return this.core;
  }

  // --- EffectHandler ------------------------------------------------------

  render(): void {
    this.setState(this.view());
  }

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

  /// `NotifyAfter` is answered exactly once, with the id of the timer that
  /// fired. If `Clear` arrives first the timeout is cancelled and the promise
  /// settles there and then, answering anyway — harmless, because the core
  /// stops listening for this request the moment it clears the timer.
  timeNotifyAfter(operation: NotifyAfter): Promise<TimerId> {
    const milliseconds = Number(operation.duration.nanos) / 1e6;
    const timerId = operation.id.value;

    return new Promise((resolve) => {
      const handle = window.setTimeout(() => {
        this.timers.delete(timerId);
        resolve(operation.id);
      }, milliseconds);
      this.timers.set(timerId, { handle, resolve });
    });
  }

  /// `Clear` is a request: drop the timer and answer with the id it named.
  timeClear(operation: Clear): Promise<TimerId> {
    const timerId = operation.id.value;
    const timer = this.timers.get(timerId);
    if (timer !== undefined) {
      window.clearTimeout(timer.handle);
      this.timers.delete(timerId);
      // Settle the `timeNotifyAfter` promise rather than leaving it pending.
      // Nothing acts on it — the core is no longer listening for that request.
      timer.resolve(operation.id);
    }

    return Promise.resolve(operation.id);
  }
}

function deserializeRequests(bytes: Uint8Array | number[]): Request[] {
  const deserializer = new BincodeDeserializer(asBytes(bytes));
  const len = deserializer.deserializeLen();
  const requests: Request[] = [];
  for (let i = 0; i < len; i++) {
    const request = Request.deserialize(deserializer);
    requests.push(request);
  }
  return requests;
}

function deserializeView(bytes: Uint8Array | number[]): ViewModel {
  return ViewModel.deserialize(new BincodeDeserializer(asBytes(bytes)));
}

function asBytes(bytes: Uint8Array | number[]): Uint8Array {
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
}
