import type { Dispatch, SetStateAction } from "react";

import { process_event, handle_response, view } from "shared/shared";
import type {
  Effect,
  Event,
  HttpResponse,
  SseResponse,
} from "shared_types/types/shared_types";
import {
  EffectVariantRender,
  ViewModel,
  EffectVariantHttp,
  EffectVariantServerSentEvents,
  Request,
} from "shared_types/types/shared_types";
import {
  BincodeSerializer,
  BincodeDeserializer,
} from "shared_types/bincode/mod";

import { request as http } from "./http";
import { request as sse } from "./sse";

type Response = HttpResponse | SseResponse;

export function update(
  event: Event,
  callback: Dispatch<SetStateAction<ViewModel>>
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
  callback: Dispatch<SetStateAction<ViewModel>>
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
    case EffectVariantServerSentEvents: {
      const request = (effect as EffectVariantServerSentEvents).value;
      for await (const response of sse(request)) {
        respond(uuid, response, callback);
      }
      break;
    }
  }
}

function respond(
  uuid: number[],
  response: Response,
  callback: Dispatch<SetStateAction<ViewModel>>
) {
  const serializer = new BincodeSerializer();
  response.serialize(serializer);

  const effects = handle_response(new Uint8Array(uuid), serializer.getBytes());

  const requests = deserializeRequests(effects);
  for (const { uuid, effect } of requests) {
    processEffect(uuid, effect, callback);
  }
}

function deserializeRequests(bytes: Uint8Array): Request[] {
  const deserializer = new BincodeDeserializer(bytes);
  const len = deserializer.deserializeLen();
  const requests: Request[] = [];
  for (let i = 0; i < len; i++) {
    const request = Request.deserialize(deserializer);
    requests.push(request);
  }
  return requests;
}

function deserializeView(bytes: Uint8Array): ViewModel {
  return ViewModel.deserialize(new BincodeDeserializer(bytes));
}
