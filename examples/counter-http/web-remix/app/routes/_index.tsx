import { useEffect, useRef, useState } from "react";

import {
  ViewModel,
  EventVariantStartWatch,
  EventVariantDecrement,
  EventVariantIncrement,
} from "shared_types/app";
import { Core } from "../core";

export const meta = () => {
  return [
    { title: "Crux Counter Example - Remix" },
    { name: "description", content: "Rust Core, TypeScript Shell (Remix)" },
  ];
};

export default function Index() {
  const [view, setView] = useState(new ViewModel("", true));
  const core: React.RefObject<Core | null> = useRef(null);

  useEffect(
    () => {
        if (core.current === null) {
          core.current = new Core(setView);

          // Initial events
          core.current?.update(new EventVariantStartWatch());
        }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ [],
  );

  return (
    <main>
      <section className="section has-text-centered">
        <p className="title">Crux Counter Example</p>
        <p className="is-size-5">Rust Core, TypeScript Shell (Remix)</p>
      </section>
      <section className="container has-text-centered">
        <p className="is-size-5">{view.text}</p>
        <div className="buttons section is-centered">
          <button
            className="button is-primary is-warning"
            onClick={() => core.current?.update(new EventVariantDecrement())}
          >
            {"Decrement"}
          </button>
          <button
            className="button is-primary is-danger"
            onClick={() => core.current?.update(new EventVariantIncrement())}
          >
            {"Increment"}
          </button>
        </div>
      </section>
    </main>
  );
}
