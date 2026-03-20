"use client";

import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useRef, useState } from "react";

import {
  ViewModel,
  EventVariantReset,
  EventVariantIncrement,
  EventVariantDecrement,
} from "shared_types/app";

import { Core } from "./core";

const Home: NextPage = () => {
  const [view, setView] = useState(new ViewModel(""));
  const core = useRef(new Core(setView));

  // Initialize
  useEffect(
    () => core.current.initialize(true),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ [],
  );

  return (
    <>
      <Head>
        <title>Next.js Counter</title>
      </Head>

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
    </>
  );
};

export default Home;
