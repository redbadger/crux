import type { Dispatch, SetStateAction } from "react";

import { process_event, view } from "{{core_name}}/{{core_name}}";
import type { Effect, Event } from "{{type_gen}}/types/{{type_gen}}";
import {
  EffectVariantRender,
  ViewModel,
  Request,
} from "{{type_gen}}/types/{{type_gen}}";
import {
  BincodeSerializer,
  BincodeDeserializer,
} from "{{type_gen}}/bincode/mod";

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

function processEffect(
  _uuid: number[],
  effect: Effect,
  callback: Dispatch<SetStateAction<ViewModel>>
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
