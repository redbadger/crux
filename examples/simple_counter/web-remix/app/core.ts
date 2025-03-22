import type { Dispatch, SetStateAction } from "react";

import { process_event, view, init } from "shared/shared";
import type { Effect, Event } from "shared_types/types/shared_types";
import {
  EffectVariantRender,
  ViewModel,
  Request,
  // EffectVariantInterval,
  EventVariantStartInterval,
  // IntervalTick,
} from "shared_types/types/shared_types";
import {
  BincodeSerializer,
  BincodeDeserializer,
} from "shared_types/bincode/mod";

export function initialize(callback: Dispatch<SetStateAction<ViewModel>>) {
  init((data: Uint8Array) => handleEffects(callback, data));
  update(new EventVariantStartInterval(), () => {});
}

function handleEffects(callback: Dispatch<SetStateAction<ViewModel>>, data: Uint8Array) {
  const requests = deserializeRequests(data);
  for (const { id, effect } of requests) {
    processEffect(id, effect, callback);
  }
}

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

function processEffect(
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
