import { process_event, view } from "shared/shared";
import type { Event } from "shared_types/types/shared_types";
import {
  EffectVariantRender,
  ViewModel,
  Request,
} from "shared_types/types/shared_types";
import {
  BincodeSerializer,
  BincodeDeserializer,
} from "shared_types/bincode/mod";

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

  for (const { effect } of requests) {
    console.log("effect", effect);
    switch (effect.constructor) {
      case EffectVariantRender: {
        callback(deserializeView(view()));
        break;
      }
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
