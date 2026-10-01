"use client";

import type { NextPage } from "next";
import { useEffect, useRef, useState } from "react";

import type { Core } from "shared_types/app";
import {
  ViewModel,
  eventStartWatch,
  eventDecrement,
  eventIncrement,
} from "shared_types/app";

import { createCore } from "./core";

const Home: NextPage = () => {
  const [view, setView] = useState(new ViewModel("", true));
  const core = useRef<Core | null>(null);
  const initialized = useRef(false);

  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;

    void createCore(setView).then((created) => {
      core.current = created;
      // `onView` is only called on a render, so show the initial view now.
      setView(created.view);
      // Initial events
      created.update(eventStartWatch());
    });
  }, []);

  return (
    <main>
      <section className="section has-text-centered">
        <p className="title">Crux Counter Example</p>
        <p className="is-size-5">Rust Core, TypeScript Shell (Next.js)</p>
      </section>
      <section className="container has-text-centered">
        <p className="is-size-5">{view.text}</p>
        <div className="buttons section is-centered">
          <button
            className="button is-primary is-warning"
            onClick={() => core.current?.update(eventDecrement())}
          >
            {"Decrement"}
          </button>
          <button
            className="button is-primary is-danger"
            onClick={() => core.current?.update(eventIncrement())}
          >
            {"Increment"}
          </button>
        </div>
      </section>
    </main>
  );
};

export default Home;
