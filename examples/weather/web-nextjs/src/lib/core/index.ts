import type { Dispatch, SetStateAction } from "react";

import { CoreFFI } from "shared";
import type { Effect, Event, RenderOperation } from "shared_types/app";
import {
  EffectVariantHttp,
  EffectVariantKeyValue,
  EffectVariantLocation,
  EffectVariantRender,
  EffectVariantSecret,
  EffectVariantTime,
  Request,
  ViewModel,
} from "shared_types/app";
import { BincodeDeserializer, BincodeSerializer } from "shared_types/bincode";
import * as http from "./http";
import * as kv from "./kv";
import * as location from "./location";
import * as secret from "./secret";
import * as time from "./time";

// union of all Operation types, only render is needed here
type Response = RenderOperation;

// ANCHOR: core_base
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
  // ANCHOR_END: core_base

  async resolve(id: number, effect: Effect) {
    switch (effect.constructor) {
      case EffectVariantRender: {
        this.callback(deserializeView(this.core.view()));
        break;
      }
      // ANCHOR: http
      case EffectVariantHttp: {
        const request = (effect as EffectVariantHttp).value;
        const response = await http.request(request);
        this.respond(id, response);
        break;
      }
      // ANCHOR_END: http
      case EffectVariantKeyValue: {
        const request = (effect as EffectVariantKeyValue).value;
        const response = await kv.handle(request);
        this.respond(id, response);
        break;
      }
      case EffectVariantLocation: {
        const request = (effect as EffectVariantLocation).value;
        const response = await location.handle(request);
        this.respond(id, response);
        break;
      }
      case EffectVariantSecret: {
        const request = (effect as EffectVariantSecret).value;
        const response = secret.handle(request);
        this.respond(id, response);
        break;
      }
      case EffectVariantTime: {
        const request = (effect as EffectVariantTime).value;
        const response = await time.handle(request);
        this.respond(id, response);
        break;
      }
    }
  }

  // ANCHOR: respond
  respond(id: number, response: Response) {
    const serializer = new BincodeSerializer();
    response.serialize(serializer);

    const effects = this.core.resolve(id, serializer.getBytes());

    const requests = deserializeRequests(effects);
    for (const { id, effect } of requests) {
      this.resolve(id, effect);
    }
  }
  // ANCHOR_END: respond
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
