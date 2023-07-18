import type { V2_MetaFunction } from "@remix-run/node";
import { useEffect, useState } from "react";

import { process_event, view } from "shared/shared";
import * as types from "shared_types/types/shared_types";
import * as bincode from "shared_types/bincode/mod";

interface Event {
  kind: "event";
  event: types.Event;
}

type State = {
  count: string;
};

const initialState: State = {
  count: "",
};

function deserialize_effects(bytes: Uint8Array) {
  let deserializer = new bincode.BincodeDeserializer(bytes);

  const len = deserializer.deserializeLen();

  let effects: types.Request[] = [];

  for (let i = 0; i < len; i++) {
    const effect = types.Request.deserialize(deserializer);
    effects.push(effect);
  }

  return effects;
}

export const meta: V2_MetaFunction = () => {
  return [
    { title: "New Remix App" },
    { name: "description", content: "Welcome to Remix!" },
  ];
};

export default function Index() {
  const [state, setState] = useState(initialState);

  const dispatch = (action: Event) => {
    const serializer = new bincode.BincodeSerializer();
    action.event.serialize(serializer);
    const effects = process_event(serializer.getBytes());
    process_effects(effects);
  };

  const process_effects = async (bytes: Uint8Array) => {
    let effects = deserialize_effects(bytes);

    for (const { effect } of effects) {
      switch (effect.constructor) {
        case types.EffectVariantRender:
          let bytes = view();
          let viewDeserializer = new bincode.BincodeDeserializer(bytes);
          let viewModel = types.ViewModel.deserialize(viewDeserializer);

          setState({
            count: viewModel.count,
          });

          break;
      }
    }
  };

  useEffect(() => {
    // Initial event
    dispatch({
      kind: "event",
      event: new types.EventVariantReset(),
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <main>
      <section className="box container has-text-centered m-5">
        <p className="is-size-5">{state.count}</p>
        <div className="buttons section is-centered">
          <button
            className="button is-primary is-danger"
            onClick={() =>
              dispatch({
                kind: "event",
                event: new types.EventVariantReset(),
              })
            }
          >
            {"Reset"}
          </button>
          <button
            className="button is-primary is-success"
            onClick={() =>
              dispatch({
                kind: "event",
                event: new types.EventVariantIncrement(),
              })
            }
          >
            {"Increment"}
          </button>
          <button
            className="button is-primary is-warning"
            onClick={() =>
              dispatch({
                kind: "event",
                event: new types.EventVariantDecrement(),
              })
            }
          >
            {"Decrement"}
          </button>
        </div>
      </section>
    </main>
  );
}
