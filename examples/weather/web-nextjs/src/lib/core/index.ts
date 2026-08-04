import type { Dispatch, SetStateAction } from "react";

import { CoreFFI } from "shared";
import type { Effect, Event, ViewModel } from "shared_types/app";
import {
  Request,
  matchEffect,
  serializeEvent,
  serializeHttpResult,
  serializeKeyValueResult,
  serializeLocationResult,
  serializeSecretResponse,
  serializeTimeResponse,
  deserializeViewModel,
} from "shared_types/app";
import type { Serializer } from "shared_types/serde";
import { BincodeDeserializer, BincodeSerializer } from "shared_types/bincode";
import * as http from "./http";
import * as kv from "./kv";
import * as location from "./location";
import * as secret from "./secret";
import * as time from "./time";

// ANCHOR: core_base
export class Core {
  core: CoreFFI;
  callback: Dispatch<SetStateAction<ViewModel>>;

  constructor(callback: Dispatch<SetStateAction<ViewModel>>) {
    this.callback = callback;
    this.core = CoreFFI.new();
  }

  update(event: Event) {
    const serializer = new BincodeSerializer();
    serializeEvent(event, serializer);

    const effects = this.core.update(serializer.getBytes());

    const requests = deserializeRequests(effects);
    for (const { id, effect } of requests) {
      this.resolve(id, effect);
    }
  }
  // ANCHOR_END: core_base

  resolve(id: number, effect: Effect) {
    matchEffect(effect, {
      Render: (): void => {
        this.callback(deserializeView(this.core.view()));
      },
      // ANCHOR: http
      Http: (e): void => {
        void http
          .request(e.value)
          .then((r) => this.respond(id, (s) => serializeHttpResult(r, s)));
      },
      // ANCHOR_END: http
      KeyValue: (e): void => {
        void kv
          .handle(e.value)
          .then((r) => this.respond(id, (s) => serializeKeyValueResult(r, s)));
      },
      Location: (e): void => {
        void location
          .handle(e.value)
          .then((r) => this.respond(id, (s) => serializeLocationResult(r, s)));
      },
      Secret: (e): void => {
        const r = secret.handle(e.value);
        this.respond(id, (s) => serializeSecretResponse(r, s));
      },
      Time: (e): void => {
        void time
          .handle(e.value)
          .then((r) => this.respond(id, (s) => serializeTimeResponse(r, s)));
      },
    });
  }

  // ANCHOR: respond
  respond(id: number, serialize: (s: Serializer) => void) {
    const serializer = new BincodeSerializer();
    serialize(serializer);

    const effects = this.core.resolve(id, serializer.getBytes());

    const requests = deserializeRequests(effects);
    for (const { id, effect } of requests) {
      this.resolve(id, effect);
    }
  }
  // ANCHOR_END: respond
}

function deserializeRequests(bytes: Uint8Array | number[]) {
  const deserializer = new BincodeDeserializer(asBytes(bytes));
  const len = deserializer.deserializeLen();
  const requests: Request[] = [];
  for (let i = 0; i < len; i++) {
    const request = Request.deserialize(deserializer);
    requests.push(request);
  }
  return requests;
}

function deserializeView(bytes: Uint8Array | number[]) {
  return deserializeViewModel(new BincodeDeserializer(asBytes(bytes)));
}

function asBytes(bytes: Uint8Array | number[]): Uint8Array {
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
}
