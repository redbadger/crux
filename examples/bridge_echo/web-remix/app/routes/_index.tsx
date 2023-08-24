import type { V2_MetaFunction } from "@remix-run/node";
import { useEffect, useState } from "react";

import {
  ViewModel,
  EventVariantReset,
  EventVariantIncrement,
  EventVariantDecrement,
} from "shared_types/types/shared_types";
import { update } from "../core";

export const meta: V2_MetaFunction = () => {
  return [
    { title: "New Remix App" },
    { name: "description", content: "Welcome to Remix!" },
  ];
};

export default function Index() {
  const [view, setView] = useState(new ViewModel("0"));

  useEffect(
    () => {
      // Initial event
      update(new EventVariantReset(), setView);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ []
  );

  return (
    <main>
      <section className="box container has-text-centered m-5">
        <p className="is-size-5">{view.count}</p>
        <div className="buttons section is-centered">
          <button
            className="button is-primary is-danger"
            onClick={() => update(new EventVariantReset(), setView)}
          >
            {"Reset"}
          </button>
          <button
            className="button is-primary is-success"
            onClick={() => update(new EventVariantIncrement(), setView)}
          >
            {"Increment"}
          </button>
          <button
            className="button is-primary is-warning"
            onClick={() => update(new EventVariantDecrement(), setView)}
          >
            {"Decrement"}
          </button>
        </div>
      </section>
    </main>
  );
}
