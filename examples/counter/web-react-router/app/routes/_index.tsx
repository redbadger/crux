import { useEffect, useRef, useState } from "react";

import {
  ViewModel,
  EventVariantReset,
  EventVariantIncrement,
  EventVariantDecrement,
} from "shared_types/app";
import { Core } from "../core";

export const meta = () => {
  return [
    { title: "Crux Counter — React Router" },
    { name: "description", content: "Crux Counter with React Router" },
  ];
};

export default function Index() {
  const [view, setView] = useState(new ViewModel(""));
  const core = useRef(new Core(setView));

  // Initialize
  useEffect(
    () =>
      core.current.initialize(/* loading is done in entry.client.tsx */ false),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ [],
  );

  return (
    <main>
      <section className="box container has-text-centered m-5">
        <p className="is-size-5">{view.count}</p>
        <div className="buttons section is-centered">
          <button
            className="button is-primary is-danger"
            onClick={() => core.current.update(new EventVariantReset())}
          >
            {"Reset"}
          </button>
          <button
            className="button is-primary is-success"
            onClick={() => core.current.update(new EventVariantIncrement())}
          >
            {"Increment"}
          </button>
          <button
            className="button is-primary is-warning"
            onClick={() => core.current.update(new EventVariantDecrement())}
          >
            {"Decrement"}
          </button>
        </div>
      </section>
    </main>
  );
}
