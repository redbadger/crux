import { useEffect, useRef, useState } from "react";

import type { Core } from "shared_types/app";
import {
  ViewModel,
  eventReset,
  eventIncrement,
  eventDecrement,
} from "shared_types/app";
import { createCore } from "../core";

export const meta = () => {
  return [
    { title: "Crux Counter — React Router" },
    { name: "description", content: "Crux Counter with React Router" },
  ];
};

export default function Index() {
  const [view, setView] = useState(new ViewModel("", 0));
  const core = useRef<Core | null>(null);
  const initialized = useRef(false);

  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;

    void createCore(setView).then((created) => {
      core.current = created;
      // `onView` is only called on a render, so show the initial view now.
      setView(created.view);
    });
  }, []);

  return (
    <main>
      <section className="box container has-text-centered m-5">
        <p className="is-size-5">{view.count}</p>
        <div className="buttons section is-centered">
          <button
            className="button is-primary is-danger"
            onClick={() => core.current?.update(eventReset())}
          >
            {"Reset"}
          </button>
          <button
            className="button is-primary is-success"
            onClick={() => core.current?.update(eventIncrement())}
          >
            {"Increment"}
          </button>
          <button
            className="button is-primary is-warning"
            onClick={() => core.current?.update(eventDecrement())}
          >
            {"Decrement"}
          </button>
        </div>
      </section>
    </main>
  );
}
