import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useState } from "react";

import init_core, {
  message as sendMessage,
  response as sendResponse,
  view,
} from "../shared/core";
import * as types from "shared_types/types/shared_types";
import * as bcs from "shared_types/bcs/mod";

interface Message {
  kind: "message";
  message: types.Event;
}

interface Response {
  kind: "response";
  uuid: number[];
  outcome: types.HttpResponse;
}

type State = {
  text: string;
};

const initialState: State = {
  text: "",
};

function deserializeRequests(bytes: Uint8Array) {
  let deserializer = new bcs.BcsDeserializer(bytes);

  const len = deserializer.deserializeLen();

  let requests: types.Request[] = [];

  for (let i = 0; i < len; i++) {
    const request = types.Request.deserialize(deserializer);
    requests.push(request);
  }

  return requests;
}

const Home: NextPage = () => {
  const [state, setState] = useState(initialState);

  const dispatch = (action: Message) => {
    const serializer = new bcs.BcsSerializer();
    action.message.serialize(serializer);
    const requests = sendMessage(serializer.getBytes());
    handleRequests(requests);
  };

  const respond = (action: Response) => {
    const serializer = new bcs.BcsSerializer();
    action.outcome.serialize(serializer);
    const moreRequests = sendResponse(
      new Uint8Array(action.uuid),
      serializer.getBytes()
    );
    handleRequests(moreRequests);
  };

  const handleRequests = async (bytes: Uint8Array) => {
    let requests = deserializeRequests(bytes);

    for (const { uuid, effect } of requests) {
      switch (effect.constructor) {
        case types.EffectVariantRender:
          let bytes = view();
          let viewDeserializer = new bcs.BcsDeserializer(bytes);
          let viewModel = types.ViewModel.deserialize(viewDeserializer);

          // core asked for a re-render with new state
          setState({
            text: viewModel.text,
          });

          break;
        case types.EffectVariantHttp:
          const { method, url } = (effect as types.EffectVariantHttp).value;
          const req = new Request(url, { method });
          const res = await fetch(req);
          const body = await res.arrayBuffer();
          const response_bytes = Array.from(new Uint8Array(body));

          respond({
            kind: "response",
            uuid,
            outcome: new types.HttpResponse(res.status, response_bytes),
          });
          break;
        default:
      }
    }
  };

  useEffect(() => {
    async function loadCore() {
      await init_core();

      // Initial message
      dispatch({
        kind: "message",
        message: new types.EventVariantGet(),
      });
    }

    loadCore();
  }, []);

  return (
    <div className="container">
      <Head>
        <title>Crux Counter Example - NextJS</title>
      </Head>

      <main>
        <section className="section title has-text-centered">
          <p>Crux Counter Example</p>
        </section>
        <section className="section container has-text-centered">
          <p>{state.text}</p>
        </section>
        <div className="buttons container is-centered">
          <button
            className="button is-primary is-warning"
            onClick={() =>
              dispatch({
                kind: "message",
                message: new types.EventVariantDecrement(),
              })
            }
          >
            {"Decrement"}
          </button>
          <button
            className="button is-primary is-danger"
            onClick={() =>
              dispatch({
                kind: "message",
                message: new types.EventVariantIncrement(),
              })
            }
          >
            {"Increment"}
          </button>
        </div>
      </main>
    </div>
  );
};

export default Home;
