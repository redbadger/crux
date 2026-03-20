import { Dispatch, RefObject, SetStateAction } from "react";
import { CoreFFI } from "shared";
import type { Effect, Event } from "shared_types/app";
import {
  EffectVariantRender,
  RenderOperation,
  Request,
  ViewModel,
} from "shared_types/app";
import { BincodeDeserializer, BincodeSerializer } from "shared_types/bincode";
import init_core from "shared/shared";

type Response = RenderOperation;

export class Core {
  core: CoreFFI | null = null;
  setState: Dispatch<SetStateAction<ViewModel>>;

  constructor(setState: Dispatch<SetStateAction<ViewModel>>) {
    // Don't initialize CoreFFI here - wait for WASM to be loaded
    this.setState = setState;
  }

  initialize(should_load: boolean) {
    if (!this.core) {
      const load = should_load ? init_core() : Promise.resolve();
      load
        .then(() => {
          this.core = new CoreFFI();
          this.setState(this.view());
        })
        .catch((error) => {
          console.error("Failed to initialize wasm core:", error);
        });
    }
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
    console.log("event", event);

    const serializer = new BincodeSerializer();
    event.serialize(serializer);

    const effects = this.core.update(serializer.getBytes());

    const requests = deserializeRequests(effects);
    for (const { id, effect } of requests) {
      this.processEffect(id, effect);
    }
  }

  private processEffect(id: number, effect: Effect) {
    console.log("effect", effect);

    switch (effect.constructor) {
      case EffectVariantRender: {
        this.setState(this.view());
        break;
      }
    }
  }
}

function deserializeRequests(bytes: Uint8Array): Request[] {
  const deserializer = new BincodeDeserializer(bytes);
  const len = deserializer.deserializeLen();
  const requests: Request[] = [];
  for (let i = 0; i < len; i++) {
    const request = Request.deserialize(deserializer);
    requests.push(request);
  }
  return requests;
}

function deserializeView(bytes: Uint8Array): ViewModel {
  return ViewModel.deserialize(new BincodeDeserializer(bytes));
}
