import type { Dispatch, SetStateAction } from "react";

import { CoreFFI } from "shared";
import type { Effect, Event, RenderOperation } from "shared_types/app";
import {
  EffectVariantHttp,
  EffectVariantRender,
  EffectVariantServerSentEvents,
  Request,
  ViewModel,
} from "shared_types/app";
import { BincodeDeserializer, BincodeSerializer } from "shared_types/bincode";
import * as http from "./http";
import * as sse from "./sse";

// union of all Operation types, only render is needed here
type Response = RenderOperation;

export class Core {
  core: CoreFFI;
  callback: Dispatch<SetStateAction<ViewModel>>;

  constructor(callback: Dispatch<SetStateAction<ViewModel>>) {
    this.callback = callback;
    this.core = new CoreFFI();
  }

  update(event: Event) {
    const serializer = new BincodeSerializer();
    event.serialize(serializer);

    const effects = this.core.update(serializer.getBytes());

    const requests = deserializeRequests(effects);
    for (const { id, effect } of requests) {
      this.resolve(id, effect);
    }
  }

  async resolve(id: number, effect: Effect) {
    switch (effect.constructor) {
      case EffectVariantRender: {
        this.callback(deserializeView(this.core.view()));
        break;
      }
      case EffectVariantHttp: {
        const request = (effect as EffectVariantHttp).value;
        const response = await http.request(request);
        this.respond(id, response);
        break;
      }
      case EffectVariantServerSentEvents: {
        const request = (effect as EffectVariantServerSentEvents).value;
        (async () => {
          for await (const response of sse.request(request)) {
            this.respond(id, response);
          }
        })();
        break;
      }
    }
  }

  respond(id: number, response: Response) {
    const serializer = new BincodeSerializer();
    response.serialize(serializer);

    const effects = this.core.resolve(id, serializer.getBytes());

    const requests = deserializeRequests(effects);
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
