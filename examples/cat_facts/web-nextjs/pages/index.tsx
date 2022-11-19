import type { NextPage } from "next";
import Head from "next/head";
import { useEffect } from "react";
import UAParser from "ua-parser-js";

import useAsyncReducer from "../hooks/use_async_reducer";

import init_core, { message, response, view } from "../shared/core";
import * as types from "shared_types/types/shared_types";
import * as bcs from "shared_types/bcs/mod";
import { Optional } from "shared_types/serde/mod";

type Action = Message | Response;

interface Message {
  kind: "message";
  message: types.Msg;
}

interface Response {
  kind: "response";
  response: types.Response;
}

type State = {
  image: Optional<types.CatImage>;
  fact: string;
  platform: string;
};

const initialState: State = {
  image: new types.CatImage(""),
  fact: "",
  platform: "",
};

function deserializeRequests(bytes: Uint8Array) {
  let deserializer = new bcs.BcsDeserializer(bytes);

  const len = deserializer.deserializeLen();

  let requests = [];

  for (let i = 0; i < len; i++) {
    const request = types.Request.deserialize(deserializer);
    requests.push(request);
  }

  return requests;
}

async function reducer(state: State, action: Action): Promise<State> {
  let serializer = new bcs.BcsSerializer();

  var bytes;
  switch (action.kind) {
    case "response":
      action.response.serialize(serializer);

      bytes = response(serializer.getBytes());

      break;
    case "message":
      action.message.serialize(serializer);

      bytes = message(serializer.getBytes());

      break;
    default:
      throw new Error();
  }

  let requests = deserializeRequests(bytes);

  for (const request of requests) {
    switch (request.body.constructor) {
      case types.RequestBodyVariantRender:
        let bytes = view();
        let viewDeserializer = new bcs.BcsDeserializer(bytes);
        let viewModel = types.ViewModel.deserialize(viewDeserializer);

        state = {
          platform: viewModel.platform,
          image: viewModel.image,
          fact: viewModel.fact,
        };

        break;
      case types.RequestBodyVariantTime:
        state = await reducer(state, {
          kind: "response",
          response: new types.Response(
            request.uuid,
            new types.ResponseBodyVariantTime(new Date().toISOString())
          ),
        });

        break;
      case types.RequestBodyVariantPlatform:
        state = await reducer(state, {
          kind: "response",
          response: new types.Response(
            request.uuid,
            new types.ResponseBodyVariantPlatform(
              new UAParser(navigator.userAgent).getBrowser().name || "Unknown"
            )
          ),
        });

        break;
      case types.RequestBodyVariantKVRead:
        break;
      case types.RequestBodyVariantKVWrite:
        break;
      case types.RequestBodyVariantHttp:
        let url = (request.body as types.RequestBodyVariantHttp).value;

        let resp = await fetch(url);
        let body = await resp.arrayBuffer();
        let response_bytes = Array.from(new Uint8Array(body));

        state = await reducer(state, {
          kind: "response",
          response: new types.Response(
            request.uuid,
            new types.ResponseBodyVariantHttp(response_bytes)
          ),
        });

        break;
      default:
    }
  }

  return state;
}

const Home: NextPage = () => {
  const [state, dispatch] = useAsyncReducer(reducer, initialState);

  useEffect(() => {
    // Seems a bad idea...?
    async function loadCore() {
      if (typeof window === undefined) {
        return;
      }

      await init_core();

      // Initial messages
      dispatch({
        kind: "message",
        message: new types.MsgVariantGetPlatform(),
      });
      dispatch({
        kind: "message",
        message: new types.MsgVariantGet(),
      });
    }

    loadCore();
  }, []);

  return (
    <div className="container">
      <Head>
        <title>Cat Facts - NextJS</title>
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
              src={state.image?.file}
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
            onClick={() =>
              dispatch({
                kind: "message",
                message: new types.MsgVariantClear(),
              })
            }
          >
            {"Clear"}
          </button>
          <button
            className="button is-primary is-success"
            onClick={() =>
              dispatch({
                kind: "message",
                message: new types.MsgVariantGet(),
              })
            }
          >
            {"Get"}
          </button>
          <button
            className="button is-primary is-warning"
            onClick={() =>
              dispatch({
                kind: "message",
                message: new types.MsgVariantFetch(),
              })
            }
          >
            {"Fetch"}
          </button>
        </div>
      </main>
    </div>
  );
};

export default Home;
