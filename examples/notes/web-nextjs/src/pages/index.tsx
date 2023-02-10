import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useState } from "react";

import Navbar from "../components/Navbar/Navbar";
import Textarea, {
  ChangeEvent,
  SelectEvent,
} from "../components/Textarea/Textarea";

import init_core, { process_event as sendEvent, view } from "../../shared/core";
import * as types from "shared_types/types/shared_types";
import * as bcs from "shared_types/bcs/mod";

interface CoreEvent {
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

  const dispatch = (action: CoreEvent) => {
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

  const [inputLog, updateLog] = useState<string[]>([]);
  const [value, setValue] = useState<string>("Hello");

  const log = (line: string): void => {
    updateLog([line, ...inputLog]);
  };

  const onChange = ({ start, end, text }: ChangeEvent): void => {
    let chars = value.split("");
    chars.splice(start, end - start, text);

    log(
      `Splice (+: ${text.length} -: ${start - end})
      (${start}..${end}) = "${text}"`
    );

    setValue(chars.join(""));
  };

  const onSelect = ({ start, end }: SelectEvent): void => {
    log(`onSelect ${start}..${end}`);
  };

  return (
    <>
      <Head>
        <title>Notes</title>
      </Head>

      <div className="min-h-screen flex flex-col">
        <Navbar title="A note" />
        <main className="flex-grow flex flex-col bg-slate-100 ">
          <div className="flex-grow basis-1 flex flex-col bg-slate-200 ">
            <Textarea
              className="p-3 flex-grow resize-none w-full focus:outline-none"
              onSelect={onSelect}
              onChange={onChange}
              value={value}
            />
          </div>
          <div className="flex-grow basis-1 overflow-scroll">
            <div className=" p-3 text-sm font-mono bg-slate-100 ">
              {inputLog.map((line) => (
                <p className="font-mono">{line}</p>
              ))}
            </div>
          </div>
        </main>
      </div>
    </>
  );
};

export default Home;
