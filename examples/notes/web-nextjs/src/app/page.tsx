"use client";

import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useRef, useState } from "react";

import Navbar from "../components/Navbar/Navbar";
import Textarea, {
  ChangeEvent,
  SelectEvent,
} from "../components/Textarea/Textarea";

import init_core from "shared/shared";
import { SyncMessage, Timers, Core } from "./core";
import {
  TextCursor,
  TextCursorVariantPosition,
  TextCursorVariantSelection,
  ViewModel,
  Message,
  EventVariantOpen,
  EventVariantReplace,
  EventVariantMoveCursor,
  EventVariantSelect,
} from "shared_types/types/shared_types";

const LOG_EDITS = false;

type Selection = {
  start: number;
  end: number;
};

function cursorToSelection(cursor: TextCursor): Selection {
  var start = 0;
  var end = 0;

  switch (cursor.constructor) {
    case TextCursorVariantPosition:
      let cursorP = cursor as TextCursorVariantPosition;

      start = Number(cursorP.value);
      end = Number(cursorP.value);
      break;
    case TextCursorVariantSelection:
      let cursorS = cursor as TextCursorVariantSelection;

      start = Number(cursorS.value.start);
      end = Number(cursorS.value.end);
      break;
  }

  return {
    start,
    end,
  };
}

const Home: NextPage = () => {
  const [view, setView] = useState<ViewModel>(
    new ViewModel("", new TextCursorVariantPosition(BigInt(0))),
  );

  const [_, setTimers] = useState<Timers>({});

  // TODO the state and channel handling should probably get
  // packaged up as a custom hook or something

  const subscriptionId = useRef<number | null>(null);
  const channel = useRef(new BroadcastChannel("crux-note"));

  const core = useRef<Core>(
    new Core(setView, setTimers, channel, subscriptionId),
  );

  const onMessage = (event: MessageEvent<SyncMessage>) => {
    let message = event.data;

    // One of the peers reset, load the initial document
    if (message.kind == "reset") {
      // Don't need to do anything...?

      return;
    } else if (message.kind == "change" && message.data != null) {
      if (subscriptionId.current == null) return;

      // Pass data into the core
      core.current.respond(subscriptionId.current, new Message(message.data));
    }
  };

  const initialized = useRef(false);
  useEffect(
    () => {
      if (!initialized.current) {
        initialized.current = true;

        init_core().then(() => {
          // Subscribe to the BroadcastChannel
          channel.current.onmessage = onMessage;

          // Open the document
          core.current.update(new EventVariantOpen());

          // Ask all peers to reset
          let message: SyncMessage = {
            kind: "reset",
          };
          channel.current.postMessage(message);
        });

        const currentChannel = channel.current;
        return () => {
          currentChannel.onmessage = null;
        };
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    /*once*/ [],
  );

  // Event handlers

  const onChange = ({ start, end, text }: ChangeEvent): void => {
    log(`onChange ${start} ${end} "${text}"`);

    core.current.update(
      new EventVariantReplace(BigInt(start), BigInt(end), text),
    );
  };

  const onSelect = ({ start, end }: SelectEvent): void => {
    log(`onSelect ${start} ${end}`);

    let event =
      start == end
        ? new EventVariantMoveCursor(BigInt(end))
        : new EventVariantSelect(BigInt(start), BigInt(end));

    core.current.update(event);
  };

  const [inputLog, updateLog] = useState<string[]>([]);
  const log = (line: string): void => {
    updateLog((log) => [line, ...log.slice(0, 100)]);
  };

  let selection = cursorToSelection(view.cursor);

  return (
    <>
      <Head>
        <title>Notes</title>
      </Head>

      <div className="min-h-screen flex flex-col bg-slate-200">
        <Navbar title="A note" />
        <main className="flex-grow flex flex-col">
          <div className="flex-grow basis-1 flex flex-col">
            <Textarea
              className="p-3 flex-grow resize-none w-full focus:outline-none"
              selectionStart={selection.start}
              selectionEnd={selection.end}
              onSelect={onSelect}
              onChange={onChange}
              value={view.text}
            />
          </div>
          {LOG_EDITS ? (
            <div className="flex-grow basis-1 overflow-scroll">
              <div className=" p-3 text-sm font-mono bg-slate-100 ">
                {inputLog.map((line, i) => (
                  <p className="font-mono" key={i}>
                    {line}
                  </p>
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
