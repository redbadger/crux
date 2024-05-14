import type { Dispatch, SetStateAction } from "react";
import UAParser from "ua-parser-js";

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
  for (const { uuid, effect } of requests) {
    processEffect(uuid, effect, callback);
  }
}

async function processEffect(
  uuid: number[],
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
      respond(uuid, response, callback);
      break;
    }
    case EffectVariantTime: {
      const now = new Date();
      const instant = new Instant(BigInt(now.getSeconds()), 0);
      const response = new TimeResponseVariantnow(instant);
      respond(uuid, response, callback);
      break;
    }
    case EffectVariantPlatform: {
      const response = new PlatformResponse(
        new UAParser(navigator.userAgent).getBrowser().name || "Unknown",
      );
      respond(uuid, response, callback);
      break;
    }
    case EffectVariantKeyValue:
      break;
  }
}

function respond(
  uuid: number[],
  response: Response,
  callback: Dispatch<SetStateAction<ViewModel>>,
) {
  const serializer = new BincodeSerializer();
  response.serialize(serializer);

  const effects = handle_response(new Uint8Array(uuid), serializer.getBytes());

  const requests = deserializeRequests(effects);
  for (const { uuid, effect } of requests) {
    processEffect(uuid, effect, callback);
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
