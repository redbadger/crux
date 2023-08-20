"use client";

import type { NextPage } from "next";
import { useEffect, useState } from "react";

import init_core from "shared/shared";
import {
  ViewModel,
  EventVariantStartWatch,
  EventVariantDecrement,
  EventVariantIncrement,
} from "shared_types/types/shared_types";

import { update } from "./core";

const Home: NextPage = () => {
  const [state, setState] = useState(new ViewModel("", false));

  useEffect(
    () => {
      async function loadCore() {
        await init_core();

        // Initial event
        update(new EventVariantStartWatch(), setState);
      }

      loadCore();
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
};

export default Home;
