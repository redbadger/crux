import type { NextPage } from "next";
import Head from "next/head";
import {
  Dispatch,
  MutableRefObject,
  SetStateAction,
  useEffect,
  useRef,
  useState,
} from "react";

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
import * as bincode from "shared_types/bincode/mod";
import { Seq } from "shared_types/serde/types";

const LOG_EDITS = false;

interface CoreEvent {
  kind: "event" | "response";
  event?: types.Event;
  response?: Response;
}

type Response = {
  uuid: number[];
  data: any;
};

type State = {
  text: string;
  selectionStart: number;
  selectionEnd: number;
};

type SyncMessage = {
  kind: "change" | "reset";
  data?: number[];
};

type Timers = {
  [key: number]: number;
};

const initialState: State = { text: "", selectionStart: 0, selectionEnd: 0 };

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

function cursorToSelection(cursor: types.TextCursor): {
  selectionStart: number;
  selectionEnd: number;
} {
  var selectionStart = 0;
  var selectionEnd = 0;

  switch (cursor.constructor) {
    case types.TextCursorVariantPosition:
      let cursorP = cursor as types.TextCursorVariantPosition;

      selectionStart = Number(cursorP.value);
      selectionEnd = Number(cursorP.value);
      break;
    case types.TextCursorVariantSelection:
      let cursorS = cursor as types.TextCursorVariantSelection;

      selectionStart = Number(cursorS.value.start);
      selectionEnd = Number(cursorS.value.end);
      break;
  }

  return { selectionStart, selectionEnd };
}

// Crux Effects

function render(setState: (state: State) => void) {
  let bytes = view();
  let viewDeserializer = new bincode.BincodeDeserializer(bytes);
  let viewModel = types.ViewModel.deserialize(viewDeserializer);

  var { selectionStart, selectionEnd } = cursorToSelection(viewModel.cursor);

  setState({
    text: viewModel.text,
    selectionStart: selectionStart,
    selectionEnd: selectionEnd,
  });
}

function timer(
  timerOp: types.TimerOperation,
  updateTimers: Dispatch<SetStateAction<Timers>>, // Might not be the best idea
  dispatch: (action: CoreEvent) => void,
  uuid: Seq<number>
) {
  switch (timerOp.constructor) {
    case types.TimerOperationVariantStart:
      let { id: startId, millis } = timerOp as types.TimerOperationVariantStart;

      let handle = window.setTimeout(() => {
        // Drop the timer
        updateTimers((ts) => {
          let { [Number(startId)]: _, ...rest } = ts;

          return rest;
        });

        dispatch({
          kind: "response",
          response: {
            uuid,
            data: new types.TimerOutputVariantFinished(startId),
          },
        });
      }, Number(millis));
      updateTimers((ts) => ({ [Number(startId)]: handle, ...ts }));

      dispatch({
        kind: "response",
        response: {
          uuid,
          data: new types.TimerOutputVariantCreated(startId),
        },
      });

      break;
    case types.TimerOperationVariantCancel:
      let { id: cancelId } = timerOp as types.TimerOperationVariantCancel;

      updateTimers((ts) => {
        let { [Number(cancelId)]: handle, ...rest } = ts;
        window.clearTimeout(handle);

        return rest;
      });
  }
}

function pubSub(
  pubSubOp: types.PubSubOperation,
  channel: MutableRefObject<BroadcastChannel>,
  subscriptionId: MutableRefObject<number[] | null>,
  uuid: Seq<number>
) {
  switch (pubSubOp.constructor) {
    case types.PubSubOperationVariantPublish:
      let publish = pubSubOp as types.PubSubOperationVariantPublish;
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

function keyValue(
  keyValueOp: types.KeyValueOperation,
  dispatch: (action: CoreEvent) => void,
  uuid: Seq<number>
) {
  switch (keyValueOp.constructor) {
    case types.KeyValueOperationVariantRead:
      const { value: readKey } =
        keyValueOp as types.KeyValueOperationVariantRead;

      let data = window.localStorage.getItem(readKey);
      let bytes: number[] | null = data == null ? data : JSON.parse(data);

      console.log(`Loaded document (${bytes?.length} bytes)`);

      dispatch({
        kind: "response",
        response: {
          uuid,
          data: new types.KeyValueOutputVariantRead(bytes),
        },
      });

      break;
    case types.KeyValueOperationVariantWrite:
      const { field0: writeKey, field1: writeValue } =
        keyValueOp as types.KeyValueOperationVariantWrite;

      console.log(`Saving document (${writeValue.length} bytes)`);

      // FIXME JSON is not exactly a space efficient format
      window.localStorage.setItem(writeKey, JSON.stringify(writeValue));

      dispatch({
        kind: "response",
        response: {
          uuid,
          data: new types.KeyValueOutputVariantWrite(true),
        },
      });

      break;
  }
}

const Home: NextPage = () => {
  const [state, setState] = useState<State>(initialState);
  const [_, updateTimers] = useState<Timers>({});

  // TODO the state and channel handling should probably get
  // packaged up as a custom hook or something

  const subscriptionId = useRef<number[] | null>(null);
  const channel = useRef(new BroadcastChannel("crux-note"));

  const dispatch = (action: CoreEvent) => {
    const serializer = new bincode.BincodeSerializer();

    if (action.kind == "event") {
      action.event?.serialize(serializer);
      const requests = sendEvent(serializer.getBytes());
      handleRequests(requests);
    } else {
      if (action.response == null) return;

      action.response?.data.serialize(serializer);
      let uuid = Uint8Array.from(action.response?.uuid);

      const requests = sendResponse(uuid, serializer.getBytes());
      handleRequests(requests);
    }
  };

  const handleRequests = async (bytes: Uint8Array) => {
    let requests = deserializeRequests(bytes);

    for (const { uuid, effect } of requests) {
      console.log("Handling effect", effect);

      switch (effect.constructor) {
        case types.EffectVariantRender:
          render(setState);

          break;
        case types.EffectVariantPubSub:
          const pubSubOp = (effect as types.EffectVariantPubSub).value;

          pubSub(pubSubOp, channel, subscriptionId, uuid);
          break;
        case types.EffectVariantTimer:
          const timerOp = (effect as types.EffectVariantTimer).value;

          timer(timerOp, updateTimers, dispatch, uuid);
          break;
        case types.EffectVariantKeyValue:
          const keyValueOp = (effect as types.EffectVariantKeyValue).value;

          keyValue(keyValueOp, dispatch, uuid);
          break;
      }
    }
  };

  const onMessage = (event: MessageEvent<SyncMessage>) => {
    let message = event.data;

    // One of the peers reset, load the initial document
    if (message.kind == "reset") {
      // Don't need to do anything...?

      return;
    } else if (message.kind == "change" && message.data != null) {
      if (subscriptionId.current == null) return;
      let data = message.data;

      // Pass data into the core
      dispatch({
        kind: "response",
        response: {
          uuid: subscriptionId.current,
          data: new types.Message(message.data),
        },
      });
    }
  };

  useEffect(() => {
    async function loadCore() {
      await init_core();

      // Subscribe to the BroadcastChannel
      channel.current.onmessage = onMessage;

      // Open the document
      dispatch({
        kind: "event",
        event: new types.EventVariantOpen(),
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
    log(`onChange ${start} ${end} "${text}"`);

    dispatch({
      kind: "event",
      event: new types.EventVariantReplace(BigInt(start), BigInt(end), text),
    });
  };

  const onSelect = ({ start, end }: SelectEvent): void => {
    log(`onSelect ${start} ${end}`);

    let event =
      start == end
        ? new types.EventVariantMoveCursor(BigInt(end))
        : new types.EventVariantSelect(BigInt(start), BigInt(end));

    dispatch({ kind: "event", event: event });
  };

  const [inputLog, updateLog] = useState<string[]>([]);
  const log = (line: string): void => {
    updateLog((log) => [line, ...log.slice(0, 100)]);
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
          {LOG_EDITS ? (
            <div className="flex-grow basis-1 overflow-scroll">
              <div className=" p-3 text-sm font-mono bg-slate-100 ">
                {inputLog.map((line) => (
                  <p className="font-mono">{line}</p>
                ))}
              </div>
            </div>
          ) : null}
        </main>
      </div>
    </>
  );
};

export default Home;
