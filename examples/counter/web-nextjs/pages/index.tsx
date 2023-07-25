import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useState } from "react";

import init_core, {
  process_event,
  handle_response,
  view,
} from "../shared/core";
import type { Event } from "shared_types/types/shared_types";
import {
  HttpResponse,
  SseResponseVariantChunk,
  SseResponseVariantDone,
  EffectVariantRender,
  ViewModel,
  EffectVariantHttp,
  EffectVariantServerSentEvents,
  EventVariantStartWatch,
  EventVariantDecrement,
  EventVariantIncrement,
  Request,
} from "shared_types/types/shared_types";
import {
  BincodeSerializer,
  BincodeDeserializer,
} from "shared_types/bincode/mod";

import { http } from "./http";
import { sse } from "./sse";

type Response = HttpResponse | SseResponseVariantChunk | SseResponseVariantDone;

const Home: NextPage = () => {
  const [state, setState] = useState(new ViewModel("", false));

  function dispatch(event: Event) {
    const serializer = new BincodeSerializer();
    event.serialize(serializer);
    const effects = process_event(serializer.getBytes());
    processEffects(effects);
  }

  function respond(uuid: number[], response: Response) {
    const serializer = new BincodeSerializer();
    response.serialize(serializer);
    const effects = handle_response(
      new Uint8Array(uuid),
      serializer.getBytes()
    );
    processEffects(effects);
  }

  async function processEffects(effects: Uint8Array) {
    const requests = deserializeRequests(effects);

    for (const { uuid, effect } of requests) {
      switch (effect.constructor) {
        case EffectVariantRender: {
          setState(deserializeView(view()));
          break;
        }
        case EffectVariantHttp: {
          const request = (effect as EffectVariantHttp).value;
          const response = await http(request);
          respond(uuid, response);
          break;
        }
        case EffectVariantServerSentEvents: {
          const request = (effect as EffectVariantServerSentEvents).value;
          for await (const response of sse(request)) {
            respond(uuid, response);
          }
          break;
        }
      }
    }
  }

  useEffect(
    () => {
      async function loadCore() {
        await init_core();

        // Initial event
        dispatch(new EventVariantStartWatch());
      }

      loadCore();
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ []
  );

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
              onClick={() => dispatch(new EventVariantDecrement())}
            >
              {"Decrement"}
            </button>
            <button
              className="button is-primary is-danger"
              onClick={() => dispatch(new EventVariantIncrement())}
            >
              {"Increment"}
            </button>
          </div>
        </section>
      </main>
    </>
  );
};

function deserializeRequests(bytes: Uint8Array) {
  const deserializer = new BincodeDeserializer(bytes);
  const len = deserializer.deserializeLen();
  const requests: Request[] = [];
  for (let i = 0; i < len; i++) {
    const request = Request.deserialize(deserializer);
    requests.push(request);
  }
  return requests;
}

function deserializeView(bytes: Uint8Array) {
  return ViewModel.deserialize(new BincodeDeserializer(bytes));
}

export default Home;
