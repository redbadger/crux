"use client";

import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useState } from "react";

import init_core from "{{core_name}}/{{core_name}}";
import {
  ViewModel,
  EventVariantReset,
  EventVariantIncrement,
  EventVariantDecrement,
} from "{{type_gen}}/types/{{type_gen}}";

import { update } from "./core";

const Home: NextPage = () => {
  const [state, setState] = useState(new ViewModel("0"));

  useEffect(
    () => {
      async function loadCore() {
        await init_core();

        // Initial event
        update(new EventVariantReset(), setState);
      }

      loadCore();
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ []
  );

  return (
    <>
      <Head>
        <title>Next.js Counter</title>
      </Head>

      <main>
        <section className="box container has-text-centered m-5">
          <p className="is-size-5">{state.count}</p>
          <div className="buttons section is-centered">
            <button
              className="button is-primary is-danger"
              onClick={() => update(new EventVariantReset(), setState)}
            >
              {"Reset"}
            </button>
            <button
              className="button is-primary is-success"
              onClick={() => update(new EventVariantIncrement(), setState)}
            >
              {"Increment"}
            </button>
            <button
              className="button is-primary is-warning"
              onClick={() => update(new EventVariantDecrement(), setState)}
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
