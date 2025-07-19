import type { Dispatch, SetStateAction } from "react";

import { CoreFFI } from "shared";
import type {
  Effect,
  Event,
  HttpResponse,
} from "shared_types/types/shared_types";
import type { SseResponse } from "shared_types/types/server_sent_events";
import {
  EffectVariantRender,
  EffectVariantHttp,
  EffectVariantServerSentEvents,
  Request,
  EffectVariantRandom,
} from "shared_types/types/shared_types";
import { ViewModel } from "shared_types/types/view_model";
import {
  BincodeSerializer,
  BincodeDeserializer,
} from "shared_types/bincode/mod";

import { request as http } from "./http";
import { request as sse } from "./sse";
import { request as random } from "./random";

type Response = HttpResponse | SseResponse;

export class Core {
  core: CoreFFI;
  callback: Dispatch<SetStateAction<ViewModel>>;

  constructor(callback: Dispatch<SetStateAction<ViewModel>>) {
    this.callback = callback;

    const self = this;
    this.core = new CoreFFI((effects: Uint8Array) => {
      const requests = deserializeRequests(effects);
      for (const { id, effect } of requests) {
        self.resolve(id, effect);
      }
    });
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
        const response = await http(request);
        this.respond(id, response);
        break;
      }
      case EffectVariantServerSentEvents: {
        const request = (effect as EffectVariantServerSentEvents).value;
        for await (const response of sse(request)) {
          this.respond(id, response);
        }
        break;
      }
      case EffectVariantRandom: {
        const request = (effect as EffectVariantRandom).value;
        const response = random(request);
        this.respond(id, response);
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
