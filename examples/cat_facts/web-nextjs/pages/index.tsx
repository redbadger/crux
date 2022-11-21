import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useState } from "react";
import UAParser from "ua-parser-js";

import init_core, {
  message as sendMessage,
  response as sendResponse,
  view,
} from "../shared/core";
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

const Home: NextPage = () => {
  const [state, setState] = useState(initialState);

  // think it would be great if responses could just be messages with an optional uuid field set.
  const dispatch = (action: Message) => {
    const serializer = new bcs.BcsSerializer();
    action.message.serialize(serializer);
    const requests = sendMessage(serializer.getBytes());
    handleRequests(requests);
  };

  const respond = (action: Response) => {
    const serializer = new bcs.BcsSerializer();
    action.response.serialize(serializer);
    const moreRequests = sendResponse(serializer.getBytes());
    handleRequests(moreRequests);
  };

  const handleRequests = async (bytes: any) => {
    let requests = deserializeRequests(bytes);

    for (const request of requests) {
      switch (request.body.constructor) {
        case types.RequestBodyVariantRender:
          let bytes = view();
          let viewDeserializer = new bcs.BcsDeserializer(bytes);
          let viewModel = types.ViewModel.deserialize(viewDeserializer);

          // core asked for a re-render with new state
          setState({
            platform: viewModel.platform,
            image: viewModel.image,
            fact: viewModel.fact,
          });

          break;
        case types.RequestBodyVariantTime:
          respond({
            kind: "response",
            response: new types.Response(
              request.uuid,
              new types.ResponseBodyVariantTime(new Date().toISOString())
            ),
          });

          break;
        case types.RequestBodyVariantPlatform:
          respond({
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
          const url = (request.body as types.RequestBodyVariantHttp).value;

          const resp = await fetch(url);
          const body = await resp.arrayBuffer();
          const response_bytes = Array.from(new Uint8Array(body));

          respond({
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
  };

  useEffect(() => {
    async function loadCore() {
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
