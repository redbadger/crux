import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useRef, useState } from "react";

import Navbar from "../components/Navbar/Navbar";
import Textarea, {
  ChangeEvent,
  SelectEvent,
} from "../components/Textarea/Textarea";

import init_core, {
  process_event as sendEvent,
  handle_response as sendResponse,
  view,
} from "../../shared/core";
import * as types from "shared_types/types/shared_types";
import * as bcs from "shared_types/bcs/mod";

// An automerge document that everyone starts with
const INITIAL_DOC = [
  133, 111, 74, 131, 22, 133, 108, 231, 0, 110, 1, 16, 105, 153, 47, 106, 29,
  169, 76, 82, 190, 222, 177, 130, 73, 131, 194, 44, 1, 11, 30, 11, 118, 113,
  127, 84, 123, 136, 33, 133, 182, 224, 41, 19, 143, 111, 203, 237, 90, 225, 18,
  35, 241, 161, 210, 92, 168, 24, 119, 178, 174, 6, 1, 2, 3, 2, 19, 2, 35, 2,
  64, 2, 86, 2, 7, 21, 6, 33, 2, 35, 2, 52, 1, 66, 2, 86, 2, 128, 1, 2, 127, 0,
  127, 1, 127, 1, 127, 0, 127, 0, 127, 7, 127, 4, 98, 111, 100, 121, 127, 0,
  127, 1, 1, 127, 4, 127, 0, 127, 0, 0,
];

interface CoreEvent {
  kind: "event" | "response";
  event?: types.Event;
  response?: any;
}

type State = {
  text: string;
  selectionStart: number;
  selectionEnd: number;
};

type SyncMessage = {
  kind: "change" | "reset";
  data?: number[];
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

  // TODO the state and channel handling should probably get
  // packaged up as a custom hook or something

  const subscriptionId = useRef<number[] | null>(null);
  const channel = useRef(new BroadcastChannel("crux-note"));

  const dispatch = (action: CoreEvent) => {
    const serializer = new bcs.BcsSerializer();

    if (action.kind == "event") {
      action.event?.serialize(serializer);
      const requests = sendEvent(serializer.getBytes());
      handleRequests(requests);
    } else {
      if (subscriptionId.current == null) return; // core has not subscribed

      action.response?.serialize(serializer);
      let uuid = Uint8Array.from(subscriptionId.current);

      const requests = sendResponse(uuid, serializer.getBytes());
      handleRequests(requests);
    }
  };

  const handleRequests = async (bytes: Uint8Array) => {
    let requests = deserializeRequests(bytes);

    for (const { uuid, effect } of requests) {
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
              let publish = op as types.PubSubOperationVariantPublish;
              let message: SyncMessage = {
                kind: "change",
                data: publish.value,
              };

              channel.current.postMessage(message);

              break;
            case types.PubSubOperationVariantSubscribe:
              subscriptionId.current = uuid;

              break;
          }
      }
    }
  };

  const onMessage = (event: MessageEvent<SyncMessage>) => {
    let message = event.data;

    // One of the peers reset, load the initial document
    if (message.kind == "reset") {
      dispatch({
        kind: "event",
        event: new types.EventVariantLoad(INITIAL_DOC),
      });

      return;
    } else if (message.kind == "change" && message.data != null) {
      let data = message.data;

      // Pass data into the core
      dispatch({
        kind: "response",
        response: new types.Message(message.data),
      });
    }
  };

  useEffect(() => {
    async function loadCore() {
      await init_core();

      // Subscribe to the BroadcastChannel
      channel.current.onmessage = onMessage;

      // Load the document
      dispatch({
        kind: "event",
        event: new types.EventVariantLoad(INITIAL_DOC),
      });

      // Ask all peers to reset
      let message: SyncMessage = {
        kind: "reset",
      };
      channel.current.postMessage(message);
    }

    loadCore();

    return () => {
      channel.current.onmessage = null;
    };
  }, []);

  // Event handlers

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
