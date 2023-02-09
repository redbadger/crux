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
  kind: "event" | "response";
  event: types.Event;
}

type State = {
  text: string;
  selectionStart: number;
  selectionEnd: number;
};

const initialState: State = { text: "", selectionStart: 0, selectionEnd: 0 };

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
  const [state, setState] = useState<State>(initialState);

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

          var selectionStart = 0;
          var selectionEnd = 0;

          switch (viewModel.cursor.constructor) {
            case types.TextCursorVariantPosition:
              let cursorP = viewModel.cursor as types.TextCursorVariantPosition;

              selectionStart = Number(cursorP.value);
              selectionEnd = Number(cursorP.value);
              break;
            case types.TextCursorVariantSelection:
              let cursorS =
                viewModel.cursor as types.TextCursorVariantSelection;

              selectionStart = Number(cursorS.value.start);
              selectionEnd = Number(cursorS.value.end);
              break;
          }

          setState({
            text: viewModel.text,
            selectionStart: selectionStart,
            selectionEnd: selectionEnd,
          });

          break;
        case types.EffectVariantPubSub:
          let op: types.PubSubOperation = (effect as types.EffectVariantPubSub)
            .value;

          switch (op.constructor) {
            case types.PubSubOperationVariantPublish:
              console.log("Publish", op);
              break;
            case types.PubSubOperationVariantSubscribe:
              console.log("Subscribe", op);
              break;
          }
      }
    }
  };

  useEffect(() => {
    async function loadCore() {
      await init_core();
    }

    loadCore();
  }, []);

  const onChange = ({ start, end, text }: ChangeEvent): void => {
    dispatch({
      kind: "event",
      event: new types.EventVariantReplace(BigInt(start), BigInt(end), text),
    });
  };

  const onSelect = ({ start, end }: SelectEvent): void => {
    let event =
      start == end
        ? new types.EventVariantMoveCursor(BigInt(end))
        : new types.EventVariantSelect(BigInt(start), BigInt(end));

    dispatch({ kind: "event", event: event });
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
              selectionStart={state.selectionStart}
              selectionEnd={state.selectionEnd}
              onSelect={onSelect}
              onChange={onChange}
              value={state.text}
            />
          </div>
        </main>
      </div>
    </>
  );
};

export default Home;
