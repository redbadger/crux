import type { V2_MetaFunction } from "@remix-run/node";
import { useEffect, useState } from "react";

import { process_event, view } from "shared/shared";
import type { Event } from "shared_types/types/shared_types";
import {
  Request,
  ViewModel,
  EffectVariantRender,
  EventVariantReset,
  EventVariantIncrement,
  EventVariantDecrement,
} from "shared_types/types/shared_types";
import {
  BincodeDeserializer,
  BincodeSerializer,
} from "shared_types/bincode/mod";

export const meta: V2_MetaFunction = () => {
  return [
    { title: "New Remix App" },
    { name: "description", content: "Welcome to Remix!" },
  ];
};

export default function Index() {
  const [state, setState] = useState(new ViewModel("0"));

  function dispatch(event: Event) {
    const serializer = new BincodeSerializer();
    event.serialize(serializer);
    const effects = process_event(serializer.getBytes());
    processEffects(effects);
  }

  async function processEffects(effects: Uint8Array) {
    const requests = deserializeRequests(effects);

    for (const { effect } of requests) {
      switch (effect.constructor) {
        case EffectVariantRender: {
          setState(deserializeView(view()));
          break;
        }
      }
    }
  }

  useEffect(
    () => {
      // Initial event
      dispatch(new EventVariantReset());
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ []
  );

  return (
    <main>
      <section className="box container has-text-centered m-5">
        <p className="is-size-5">{state.count}</p>
        <div className="buttons section is-centered">
          <button
            className="button is-primary is-danger"
            onClick={() => dispatch(new EventVariantReset())}
          >
            {"Reset"}
          </button>
          <button
            className="button is-primary is-success"
            onClick={() => dispatch(new EventVariantIncrement())}
          >
            {"Increment"}
          </button>
          <button
            className="button is-primary is-warning"
            onClick={() => dispatch(new EventVariantDecrement())}
          >
            {"Decrement"}
          </button>
        </div>
      </section>
    </main>
  );
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
