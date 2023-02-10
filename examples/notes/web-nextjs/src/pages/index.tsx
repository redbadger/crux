import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useState } from "react";

import Navbar from "../components/Navbar/Navbar";

import init_core, { process_event as sendEvent, view } from "../../shared/core";
import * as types from "shared_types/types/shared_types";
import * as bcs from "shared_types/bcs/mod";

interface Event {
  kind: "event";
  event: types.Event;
}

type State = {};

const initialState: State = {};

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

  const dispatch = (action: Event) => {
    const serializer = new bcs.BcsSerializer();
    action.event.serialize(serializer);
    const requests = sendEvent(serializer.getBytes());
    handleRequests(requests);
  };

  const handleRequests = async (bytes: Uint8Array) => {
    let requests = deserializeRequests(bytes);

    for (const { uuid: _, effect } of requests) {
      switch (effect.constructor) {
        case types.EffectVariantRender:
          let bytes = view();
          let viewDeserializer = new bcs.BcsDeserializer(bytes);
          let viewModel = types.ViewModel.deserialize(viewDeserializer);

          setState({});

          break;
      }
    }
  };

  useEffect(() => {
    async function loadCore() {
      await init_core();

      // Initial event
      // dispatch({
      //   kind: "event",
      //   event: TODO
      // });
    }

    loadCore();
  }, []);

  return (
    <>
      <Head>
        <title>Notes</title>
      </Head>

      <div className="min-h-screen flex flex-col">
        <Navbar title="A note" />
        <main className="flex-grow flex bg-slate-200 ">
          <textarea className="p-3 flex-grow border-none focus:outline-none">
            Hello
          </textarea>
        </main>
      </div>
    </>
  );
};

export default Home;
