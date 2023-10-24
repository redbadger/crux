"use client";

import type { NextPage } from "next";
import { useEffect, useRef, useState } from "react";

import init_core from "shared/shared";
import {
  ViewModel,
  EventVariantStartWatch,
  EventVariantDecrement,
  EventVariantIncrement,
} from "shared_types/types/shared_types";

import { update } from "./core";

const Home: NextPage = () => {
  const [view, setView] = useState(new ViewModel("", false));

  const initialized = useRef(false);
  useEffect(
    () => {
      if (!initialized.current) {
        initialized.current = true;

        init_core().then(() => {
          update(new EventVariantStartWatch(), setView);
        });
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ []
  );

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
            onClick={() => update(new EventVariantDecrement(), setView)}
          >
            {"Decrement"}
          </button>
          <button
            className="button is-primary is-danger"
            onClick={() => update(new EventVariantIncrement(), setView)}
          >
            {"Increment"}
          </button>
        </div>
      </section>
    </main>
  );
};

export default Home;
