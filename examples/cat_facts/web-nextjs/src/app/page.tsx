"use client";

import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useRef, useState } from "react";

import init_core from "shared/shared";
import {
  CatImage,
  ViewModel,
  EventVariantGetPlatform,
  EventVariantGet,
  EventVariantClear,
  EventVariantFetch,
} from "shared_types/app";

import { Core } from "./core";

const Home: NextPage = () => {
  const [view, setView] = useState(new ViewModel("", new CatImage(""), ""));
  const core: React.RefObject<Core | null> = useRef(null);

  const initialized = useRef(false);
  useEffect(
    () => {
      if (!initialized.current) {
        initialized.current = true;

        init_core().then(() => {
          if (core.current === null) {
            core.current = new Core(setView);
          }
          // Initial events
          core.current?.update(new EventVariantGetPlatform());
          core.current?.update(new EventVariantGet());
        });
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ [],
  );

  return (
    <div className="container">
      <Head>
        <title>Cat Facts - Next.js</title>
      </Head>

      <main>
        <section className="section title has-text-centered">
          <p>{view.platform}</p>
        </section>
        <section className="section container has-text-centered">
          {view.image?.href && (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              alt="A funny cat. Or at least a cute one."
              src={view.image?.href}
              style={{ height: "200px" }}
            />
          )}
        </section>
        <section className="section container has-text-centered">
          <p>{view.fact}</p>
        </section>
        <div className="buttons container is-centered">
          <button
            className="button is-primary is-danger"
            onClick={() => core.current?.update(new EventVariantClear())}
          >
            {"Clear"}
          </button>
          <button
            className="button is-primary is-success"
            onClick={() => core.current?.update(new EventVariantGet())}
          >
            {"Get"}
          </button>
          <button
            className="button is-primary is-warning"
            onClick={() => core.current?.update(new EventVariantFetch())}
          >
            {"Fetch"}
          </button>
        </div>
      </main>
    </div>
  );
};

export default Home;
