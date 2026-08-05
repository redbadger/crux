import type { Dispatch, SetStateAction } from "react";

import { CoreFfi } from "shared";
import type { Effect, Event } from "shared_types/app";
import {
  RandomNumber,
  Request,
  ViewModel,
  matchEffect,
  serializeEvent,
  serializeHttpResult,
  serializeSseResponse,
} from "shared_types/app";
import { BincodeDeserializer, BincodeSerializer } from "shared_types/bincode";
import type { Serializer } from "shared_types/serde";
import * as http from "./http";
import * as sse from "./sse";

export class Core {
  core: CoreFfi;
  callback: Dispatch<SetStateAction<ViewModel>>;

  constructor(callback: Dispatch<SetStateAction<ViewModel>>) {
    this.callback = callback;
    this.core = CoreFfi.new({
      processEffects: (bytes: Uint8Array) => {
        this.processEffects(bytes);
      },
    });
  }

  processEffects(bytes: Uint8Array) {
    const requests = deserializeRequests(bytes);
    for (const { id, effect } of requests) {
      this.resolve(id, effect);
    }
  }

  update(event: Event) {
    const serializer = new BincodeSerializer();
    serializeEvent(event, serializer);
    const effects = this.core.update(serializer.getBytes());
    const requests = deserializeRequests(new Uint8Array(effects));
    for (const { id, effect } of requests) {
      this.resolve(id, effect);
    }
  }

  resolve(id: number, effect: Effect): void {
    matchEffect(effect, {
      Render: (): void => {
        this.callback(deserializeView(new Uint8Array(this.core.view())));
      },
      Http: (e): void => {
        void http
          .request(e.value)
          .then((r) => this.respond(id, (s) => serializeHttpResult(r, s)));
      },
      ServerSentEvents: (e): void => {
        void (async () => {
          for await (const r of sse.request(e.value)) {
            this.respond(id, (s) => serializeSseResponse(r, s));
          }
        })();
      },
      Random: (e): void => {
        const min = Number(e.value.field0);
        const max = Number(e.value.field1);
        const result = Math.floor(Math.random() * (max - min)) + min;
        this.respond(id, (s) => new RandomNumber(BigInt(result)).serialize(s));
      },
    });
  }

  respond(id: number, serialize: (s: Serializer) => void) {
    const serializer = new BincodeSerializer();
    serialize(serializer);
    const effects = this.core.resolve(id, serializer.getBytes());
    const requests = deserializeRequests(new Uint8Array(effects));
    for (const { id, effect } of requests) {
      this.resolve(id, effect);
    }
  }
}

function deserializeRequests(bytes: Uint8Array) {
  const deserializer = new BincodeDeserializer(bytes);
  const len = deserializer.deserializeLen();
  const requests: Request[] = [];
  for (let i = 0; i < len; i++) {
    const request = Request.deserialize(deserializer);
    requests.push(request);
  }
  return requests;
}

function deserializeView(bytes: Uint8Array) {
  return ViewModel.deserialize(new BincodeDeserializer(bytes));
}
