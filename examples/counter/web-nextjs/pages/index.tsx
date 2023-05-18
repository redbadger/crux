import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useState } from "react";

import init_core, {
  process_event as sendEvent,
  handle_response as sendResponse,
  view,
} from "../shared/core";
import * as types from "shared_types/types/shared_types";
import * as bincode from "shared_types/bincode/mod";

interface Event {
  kind: "event";
  event: types.Event;
}

interface Response {
  kind: "response";
  uuid: number[];
  outcome:
  | types.HttpResponse
  | types.SseResponseVariantChunk
  | types.SseResponseVariantDone;
}

type State = {
  text: string;
};

const initialState: State = {
  text: "",
};

function deserializeRequests(bytes: Uint8Array) {
  let deserializer = new bincode.BincodeDeserializer(bytes);

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

  const dispatch = (action: Event) => {
    const serializer = new bincode.BincodeSerializer();
    action.event.serialize(serializer);
    const requests = sendEvent(serializer.getBytes());
    handleRequests(requests);
  };

  const respond = (action: Response) => {
    const serializer = new bincode.BincodeSerializer();
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
          let viewDeserializer = new bincode.BincodeDeserializer(bytes);
          let viewModel = types.ViewModel.deserialize(viewDeserializer);

          // core asked for a re-render with new state
          setState({
            text: viewModel.text,
          });

          break;
        case types.EffectVariantHttp: {
          const { method, url, headers } = (effect as types.EffectVariantHttp)
            .value;
          const req = new Request(url, {
            method,
            headers: headers.map((header) => [header.name, header.value]),
          });
          const res = await fetch(req);
          const body = await res.arrayBuffer();
          const response_bytes = Array.from(new Uint8Array(body));

          respond({
            kind: "response",
            uuid,
            outcome: new types.HttpResponse(res.status, response_bytes),
          });
          break;
        }
        case types.EffectVariantServerSentEvents: {
          const { url } = (effect as types.EffectVariantServerSentEvents).value;
          const req = new Request(url);
          const res = await fetch(req);
          const reader = await res.body.getReader();
          try {
            while (true) {
              const { done, value: chunk } = await reader.read();
              respond({
                kind: "response",
                uuid,
                outcome: done
                  ? new types.SseResponseVariantDone()
                  : new types.SseResponseVariantChunk(Array.from(chunk)),
              });
              if (done) {
                break;
              }
            }
          } finally {
            reader.releaseLock();
          }

          break;
        }
        default:
      }
    }
  };

  useEffect(() => {
    async function loadCore() {
      await init_core();

      // Initial event
      dispatch({
        kind: "event",
        event: new types.EventVariantStartWatch(),
      });
    }

    loadCore();
  }, []);

  return (
    <>
      <Head>
        <title>Crux Counter Example - Next.js</title>
      </Head>

      <main>
        <section className="section has-text-centered">
          <p className="title">Crux Counter Example</p>
        </section>
        <section className="section has-text-centered">
          <p className="is-size-5">Rust Core, TypeScript Shell (Next.js)</p>
        </section>
        <section className="container has-text-centered">
          <p className="is-size-5">{state.text}</p>
          <div className="buttons section is-centered">
            <button
              className="button is-primary is-warning"
              onClick={() =>
                dispatch({
                  kind: "event",
                  event: new types.EventVariantDecrement(),
                })
              }
            >
              {"Decrement"}
            </button>
            <button
              className="button is-primary is-danger"
              onClick={() =>
                dispatch({
                  kind: "event",
                  event: new types.EventVariantIncrement(),
                })
              }
            >
              {"Increment"}
            </button>
          </div>
        </section>
      </main>
    </>
  );
};

export default Home;
