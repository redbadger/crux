import { process_event, handle_response, view } from "shared/shared";
import type {
  Event,
  HttpResponse,
  KeyValueOutput,
} from "shared_types/types/shared_types";
import {
  PlatformResponse,
  TimeResponse,
} from "shared_types/types/shared_types";
import {
  EffectVariantRender,
  ViewModel,
  EffectVariantHttp,
  Request,
  EffectVariantTime,
  EffectVariantKeyValue,
  EffectVariantPlatform,
} from "shared_types/types/shared_types";
import {
  BincodeSerializer,
  BincodeDeserializer,
} from "shared_types/bincode/mod";

import { request as http } from "./http";
import UAParser from "ua-parser-js";

type Response = PlatformResponse | TimeResponse | HttpResponse | KeyValueOutput;

export function update(
  event: Event,
  callback: React.Dispatch<React.SetStateAction<ViewModel>>
) {
  console.log("event", event);
  const serializer = new BincodeSerializer();
  event.serialize(serializer);
  const effects = process_event(serializer.getBytes());
  processEffects(effects, callback);
}

async function processEffects(
  effects: Uint8Array,
  callback: React.Dispatch<React.SetStateAction<ViewModel>>
) {
  const requests = deserializeRequests(effects);

  for (const { uuid, effect } of requests) {
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
        const response = new TimeResponse(new Date().toISOString());
        respond(uuid, response, callback);
        break;
      }
      case EffectVariantPlatform: {
        const response = new PlatformResponse(
          new UAParser(navigator.userAgent).getBrowser().name || "Unknown"
        );
        respond(uuid, response, callback);
        break;
      }
      case EffectVariantKeyValue:
        break;
    }
  }
}

function respond(
  uuid: number[],
  response: Response,
  callback: React.Dispatch<React.SetStateAction<ViewModel>>
) {
  const serializer = new BincodeSerializer();
  response.serialize(serializer);
  const effects = handle_response(new Uint8Array(uuid), serializer.getBytes());
  processEffects(effects, callback);
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
