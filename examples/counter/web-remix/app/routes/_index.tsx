import type { V2_MetaFunction } from "@remix-run/node";
import { useEffect, useState } from "react";

import {
  ViewModel,
  EventVariantStartWatch,
  EventVariantDecrement,
  EventVariantIncrement,
} from "shared_types/types/shared_types";
import { update } from "../core";

export const meta: V2_MetaFunction = () => {
  return [
    { title: "Crux Counter Example - Remix" },
    { name: "description", content: "Rust Core, TypeScript Shell (Remix)" },
  ];
};

export default function Index() {
  const [state, setState] = useState(new ViewModel("", false));
  useEffect(
    () => {
      // Initial event, beware of StrictMode in ../entry.client.tsx as it will run twice in dev
      update(new EventVariantStartWatch(), setState);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ []
  );

  return (
    <main>
      <section className="section has-text-centered">
        <p className="title">Crux Counter Example</p>
        <p className="is-size-5">Rust Core, TypeScript Shell (Remix)</p>
      </section>
      <section className="container has-text-centered">
        <p className="is-size-5">{state.text}</p>
        <div className="buttons section is-centered">
          <button
            className="button is-primary is-warning"
            onClick={() => update(new EventVariantDecrement(), setState)}
          >
            {"Decrement"}
          </button>
          <button
            className="button is-primary is-danger"
            onClick={() => update(new EventVariantIncrement(), setState)}
          >
            {"Increment"}
          </button>
        </div>
      </section>
    </main>
  );
}
