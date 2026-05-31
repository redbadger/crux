import type { Dispatch, SetStateAction } from "react";
import { CoreFFI } from "shared";
import * as sharedWasm from "shared";
import type { Effect, Event } from "shared_types/app";
import { EffectVariantRender, Request, ViewModel } from "shared_types/app";
import { BincodeDeserializer, BincodeSerializer } from "shared_types/bincode";

const wasmInitialized = (sharedWasm as unknown as { initialized: Promise<void> })
  .initialized;

export class Core {
  core: CoreFFI | null = null;
  initializing: Promise<void> | null = null;
  setState: Dispatch<SetStateAction<ViewModel>>;

  constructor(setState: Dispatch<SetStateAction<ViewModel>>) {
    // Don't initialize CoreFFI here - wait for WASM to be loaded
    this.setState = setState;
  }

  initialize(shouldLoad: boolean): Promise<void> {
    if (this.core) {
      return Promise.resolve();
    }

    if (!this.initializing) {
      const load = shouldLoad ? wasmInitialized : Promise.resolve();

      this.initializing = load
        .then(() => {
          this.core = CoreFFI.new();
          this.setState(this.view());
        })
        .catch((error) => {
          this.initializing = null;
          console.error("Failed to initialize wasm core:", error);
        });
    }

    return this.initializing;
  }

  view(): ViewModel {
    if (!this.core) {
      throw new Error("Core not initialized. Call initialize() first.");
    }
    return deserializeView(this.core.view());
  }

  update(event: Event) {
    if (!this.core) {
      throw new Error("Core not initialized. Call initialize() first.");
    }
    const serializer = new BincodeSerializer();
    event.serialize(serializer);

    const effects = this.core.update(serializer.getBytes());

    const requests = deserializeRequests(effects);
    for (const { effect } of requests) {
      this.processEffect(effect);
    }
  }

  private processEffect(effect: Effect) {
    switch (effect.constructor) {
      case EffectVariantRender: {
        this.setState(this.view());
        break;
      }
    }
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
