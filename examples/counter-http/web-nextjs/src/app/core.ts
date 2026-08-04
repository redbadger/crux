import type { Dispatch, SetStateAction } from "react";

import { CoreFFI } from "shared";
import type { Effect, Event } from "shared_types/app";
import {
  matchEffect,
  serializeEvent,
  serializeHttpResult,
  serializeSseResponse,
  Request,
  ViewModel,
} from "shared_types/app";
import { BincodeDeserializer, BincodeSerializer } from "shared_types/bincode";
import type { Serializer } from "shared_types/serde";
import * as http from "./http";
import * as sse from "./sse";

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

  resolve(id: number, effect: Effect) {
    matchEffect(effect, {
      Render: (): void => {
        this.callback(deserializeView(this.core.view()));
      },
      Http: (e): void => {
        void http
          .request(e.value)
          .then((response) =>
            this.respond(id, (s) => serializeHttpResult(response, s)),
          );
      },
      ServerSentEvents: (e): void => {
        void (async () => {
          for await (const response of sse.request(e.value)) {
            this.respond(id, (s) => serializeSseResponse(response, s));
          }
        })();
      },
    });
  }

  respond(id: number, serialize: (s: Serializer) => void) {
    const serializer = new BincodeSerializer();
    serialize(serializer);

    const effects = this.core.resolve(id, serializer.getBytes());

    const requests = deserializeRequests(effects);
    for (const { id, effect } of requests) {
      this.resolve(id, effect);
    }
  }
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
  return ViewModel.deserialize(new BincodeDeserializer(asBytes(bytes)));
}

function asBytes(bytes: Uint8Array | number[]): Uint8Array {
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
}
