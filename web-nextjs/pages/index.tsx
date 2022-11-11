import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useReducer } from "react";
import * as types from "../../shared_types/generated/typescript/types/shared";
import * as bcs from "../../shared_types/generated/typescript/bcs/mod";

enum Action {
  Init,
  Clear,
  Get,
  Fetch,
}

type Message = {
  action: Action;
  core?: typeof import("../pkg/shared");
};

type State = {
  core?: any;
  image: {
    file: string;
  };
  fact: string;
  platform: string;
};

const initialState = {
  image: { file: "" },
  fact: "",
  platform: "",
};

function reducer(state: State, message: Message) {
  let serializer = new bcs.BcsSerializer();
  switch (message.action) {
    case Action.Init:
      let core = message.core;
      new types.MsgVariantGet().serialize(serializer);
      let bytes = core?.message(serializer.getBytes());

      return { core: message.core, ...state };
    case Action.Get:
    case Action.Fetch:
    case Action.Clear:
      break;
    default:
      throw new Error();
  }

  return state;
}

const Home: NextPage = () => {
  const [state, dispatch] = useReducer(reducer, initialState);

  useEffect(() => {
    // Seems a bad idea.
    async function loadCore() {
      const core = await import("../pkg/shared");
      dispatch({ action: Action.Init, core: core });
    }

    if (!state.core) {
      loadCore();
    }
  });

  return (
    <div className="container">
      <Head>
        <title>Cat Facts - NextJS</title>
        <link rel="icon" href="/favicon.ico" />
        <link
          rel="stylesheet"
          href="https://cdn.jsdelivr.net/npm/bulma@0.9.4/css/bulma.min.css"
        />
      </Head>

      <main>
        {!state.core ? (
          <p></p>
        ) : (
          <>
            <section className="section title has-text-centered">
              <p>{state.platform}</p>
            </section>
            <section className="section container has-text-centered">
              <img src={state.image.file} style={{ height: "400px" }} />
            </section>
            <section className="section container has-text-centered">
              <p>{state.fact}</p>
            </section>
            <div className="buttons container is-centered">
              <button
                className="button is-primary is-danger"
                onClick={() => dispatch({ action: Action.Clear })}
              >
                {"Clear"}
              </button>
              <button
                className="button is-primary is-success"
                onClick={() => dispatch({ action: Action.Get })}
              >
                {"Get"}
              </button>
              <button
                className="button is-primary is-warning"
                onClick={() => dispatch({ action: Action.Fetch })}
              >
                {"Fetch"}
              </button>
            </div>
          </>
        )}
      </main>
    </div>
  );
};

export default Home;
