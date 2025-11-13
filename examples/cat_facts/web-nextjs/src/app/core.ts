import type { Dispatch, SetStateAction } from "react";

import { CoreFFI } from "shared";
import type { Effect, Event, RenderOperation } from "shared_types/app";
import {
  EffectVariantHttp,
  EffectVariantKeyValue,
  EffectVariantPlatform,
  EffectVariantRender,
  EffectVariantTime,
  Instant,
  PlatformResponse,
  Request,
  TimeResponseVariantNow,
  ViewModel,
} from "shared_types/app";
import { BincodeSerializer, BincodeDeserializer } from "shared_types/bincode";
import * as http from "./http";
import { UAParser } from "ua-parser-js";

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
      case EffectVariantTime: {
        const now = new Date();
        const milliseconds = now.getTime();
        const seconds = Math.floor(milliseconds / 1000);
        const nanoseconds = Math.floor((milliseconds % 1000) * 1e6);
        const instant = new Instant(BigInt(seconds), nanoseconds);
        const response = new TimeResponseVariantNow(instant);
        this.respond(id, response);
        break;
      }
      case EffectVariantPlatform: {
        const response = new PlatformResponse(
          new UAParser(navigator.userAgent).getBrowser().name || "Unknown",
        );
        this.respond(id, response);
        break;
      }
      case EffectVariantKeyValue:
        break;
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
