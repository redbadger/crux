import type { Dispatch, SetStateAction } from "react";
import { UAParser } from "ua-parser-js";

import { handle_response, process_event, view } from "shared/shared";
import {
  BincodeDeserializer,
  BincodeSerializer,
} from "shared_types/bincode/mod";
import {
  Effect,
  EffectVariantHttp,
  EffectVariantKeyValue,
  EffectVariantPlatform,
  EffectVariantRender,
  EffectVariantTime,
  Event,
  HttpResponse,
  Instant,
  KeyValueResponse,
  PlatformResponse,
  Request,
  TimeResponse,
  TimeResponseVariantnow,
  ViewModel,
} from "shared_types/types/shared_types";

import { request as http } from "./http";

type Response =
  | PlatformResponse
  | TimeResponse
  | HttpResponse
  | KeyValueResponse;

export function update(
  event: Event,
  callback: Dispatch<SetStateAction<ViewModel>>,
) {
  console.log("event", event);

  const serializer = new BincodeSerializer();
  event.serialize(serializer);

  const effects = process_event(serializer.getBytes());

  const requests = deserializeRequests(effects);
  for (const { id, effect } of requests) {
    processEffect(id, effect, callback);
  }
}

async function processEffect(
  id: number,
  effect: Effect,
  callback: Dispatch<SetStateAction<ViewModel>>,
) {
  console.log("effect", effect);

  switch (effect.constructor) {
    case EffectVariantRender: {
      callback(deserializeView(view()));
      break;
    }
    case EffectVariantHttp: {
      const request = (effect as EffectVariantHttp).value;
      const response = await http(request);
      respond(id, response, callback);
      break;
    }
    case EffectVariantTime: {
      const now = new Date();
      const millis = now.getTime();
      const seconds = Math.floor(millis / 1000);
      const nanos = Math.floor((millis % 1000) * 1e6);
      const instant = new Instant(BigInt(seconds), nanos);
      const response = new TimeResponseVariantnow(instant);
      respond(id, response, callback);
      break;
    }
    case EffectVariantPlatform: {
      const response = new PlatformResponse(
        new UAParser(navigator.userAgent).getBrowser().name || "Unknown",
      );
      respond(id, response, callback);
      break;
    }
    case EffectVariantKeyValue:
      break;
  }
}

function respond(
  id: number,
  response: Response,
  callback: Dispatch<SetStateAction<ViewModel>>,
) {
  const serializer = new BincodeSerializer();
  response.serialize(serializer);

  const effects = handle_response(id, serializer.getBytes());

  const requests = deserializeRequests(effects);
  for (const { id, effect } of requests) {
    processEffect(id, effect, callback);
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
