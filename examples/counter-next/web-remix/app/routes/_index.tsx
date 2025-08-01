import { useEffect, useRef, useState } from "react";

import {
  EventVariantStartWatch,
  EventVariantDecrement,
  EventVariantIncrement,
  EventVariantRandom,
} from "app/app";
import { ViewModel } from "app/view_model";
import { Core } from "../core";

export const meta = () => {
  return [
    { title: "Crux Counter Example - Remix" },
    { name: "description", content: "Rust Core, TypeScript Shell (Remix)" },
  ];
};

export default function Index() {
  const [view, setView] = useState(new ViewModel("", false));

  const core: React.RefObject<Core | null> = useRef(null);
  useEffect(
    () => {
      // There may be a nicer way using https://react.dev/reference/react/useSyncExternalStore
      if (core.current === null) {
        core.current = new Core(setView);
        core.current.update(new EventVariantStartWatch());
      }
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
        <p className="is-size-5">{view.text}</p>
        <div className="section">
          <p className="buttons is-centered">
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
          </p>
          <p className="buttons is-centered">
            <button
              className="button is-primary info "
              onClick={() => core.current?.update(new EventVariantRandom())}
            >
              {"I'm feeling lucky"}
            </button>
          </p>
        </div>
      </section>
    </main>
  );
}
