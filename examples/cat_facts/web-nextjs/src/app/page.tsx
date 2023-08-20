"use client";

import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useState } from "react";

import init_core from "shared/shared";
import {
  CatImage,
  ViewModel,
  EventVariantGetPlatform,
  EventVariantGet,
  EventVariantClear,
  EventVariantFetch,
} from "shared_types/types/shared_types";

import { update } from "./core";

const Home: NextPage = () => {
  const [state, setState] = useState(new ViewModel("", new CatImage(""), ""));

  useEffect(
    () => {
      async function loadCore() {
        await init_core();

        // Initial events
        update(new EventVariantGetPlatform(), setState);
        update(new EventVariantGet(), setState);
      }

      loadCore();
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ []
  );

  return (
    <div className="container">
      <Head>
        <title>Cat Facts - Next.js</title>
      </Head>

      <main>
        <section className="section title has-text-centered">
          <p>{state.platform}</p>
        </section>
        <section className="section container has-text-centered">
          {state.image && (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              alt="A funny cat. Or at least a cute one."
              src={state.image?.href}
              style={{ height: "200px" }}
            />
          )}
        </section>
        <section className="section container has-text-centered">
          <p>{state.fact}</p>
        </section>
        <div className="buttons container is-centered">
          <button
            className="button is-primary is-danger"
            onClick={() => update(new EventVariantClear(), setState)}
          >
            {"Clear"}
          </button>
          <button
            className="button is-primary is-success"
            onClick={() => update(new EventVariantGet(), setState)}
          >
            {"Get"}
          </button>
          <button
            className="button is-primary is-warning"
            onClick={() => update(new EventVariantFetch(), setState)}
          >
            {"Fetch"}
          </button>
        </div>
      </main>
    </div>
  );
};

export default Home;
