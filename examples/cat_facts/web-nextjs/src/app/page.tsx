"use client";

import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useState } from "react";
import UAParser from "ua-parser-js";

import init_core, { process_event, handle_response, view } from "shared/shared";
import type { Event } from "shared_types/types/shared_types";
import {
  PlatformResponse,
  TimeResponse,
  HttpResponse,
  KeyValueOutput,
  CatImage,
  Request,
  EffectVariantRender,
  ViewModel,
  EffectVariantTime,
  EffectVariantPlatform,
  EffectVariantKeyValue,
  EffectVariantHttp,
  EventVariantGetPlatform,
  EventVariantGet,
  EventVariantClear,
  EventVariantFetch,
} from "shared_types/types/shared_types";
import {
  BincodeSerializer,
  BincodeDeserializer,
} from "shared_types/bincode/mod";
import { request as http } from "./http";

type Response = PlatformResponse | TimeResponse | HttpResponse | KeyValueOutput;

const Home: NextPage = () => {
  const [state, setState] = useState(new ViewModel("", new CatImage(""), ""));

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
        case EffectVariantTime: {
          const response = new TimeResponse(new Date().toISOString());
          respond(uuid, response);
          break;
        }
        case EffectVariantPlatform: {
          const response = new PlatformResponse(
            new UAParser(navigator.userAgent).getBrowser().name || "Unknown"
          );
          respond(uuid, response);
          break;
        }
        case EffectVariantKeyValue:
          break;
        case EffectVariantHttp: {
          const request = (effect as EffectVariantHttp).value;
          const response = await http(request);
          respond(uuid, response);
          break;
        }
      }
    }
  }

  useEffect(
    () => {
      async function loadCore() {
        await init_core();

        // Initial events
        dispatch(new EventVariantGetPlatform());
        dispatch(new EventVariantGet());
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
            onClick={() => dispatch(new EventVariantClear())}
          >
            {"Clear"}
          </button>
          <button
            className="button is-primary is-success"
            onClick={() => dispatch(new EventVariantGet())}
          >
            {"Get"}
          </button>
          <button
            className="button is-primary is-warning"
            onClick={() => dispatch(new EventVariantFetch())}
          >
            {"Fetch"}
          </button>
        </div>
      </main>
    </div>
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
