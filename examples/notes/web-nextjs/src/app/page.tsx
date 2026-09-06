"use client";

import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useRef, useState } from "react";

import Navbar from "../components/Navbar/Navbar";
import Textarea, {
  ChangeEvent,
  SelectEvent,
} from "../components/Textarea/Textarea";

import * as sharedWasm from "shared";
import { SyncMessage, Core } from "./core";
import type { EffectSink } from "shared_types/app";
import {
  TextCursor,
  matchTextCursor,
  textCursorPosition,
  ViewModel,
  Message,
  eventOpen,
  eventReplace,
  eventMoveCursor,
  eventSelect,
} from "shared_types/app";

const LOG_EDITS = false;

const wasmInitialized = (
  sharedWasm as unknown as { initialized: Promise<void> }
).initialized;

type Selection = {
  start: number;
  end: number;
};

function cursorToSelection(cursor: TextCursor): Selection {
  return matchTextCursor(cursor, {
    Position: (c) => ({ start: Number(c.value), end: Number(c.value) }),
    Selection: (c) => ({
      start: Number(c.value.start),
      end: Number(c.value.end),
    }),
  });
}

const Home: NextPage = () => {
  const [view, setView] = useState<ViewModel>(
    new ViewModel("", textCursorPosition(BigInt(0))),
  );

  // TODO the state and channel handling should probably get
  // packaged up as a custom hook or something

  // Set by the core's `subscribe` handler; every peer message becomes one
  // item on this sink.
  const subscription = useRef<EffectSink<Message> | null>(null);
  const channel = useRef(new BroadcastChannel("crux-note"));
  const core = useRef(new Core(setView, channel, subscription));

  const onMessage = (event: MessageEvent<SyncMessage>) => {
    let message = event.data;

    // One of the peers reset, load the initial document
    if (message.kind == "reset") {
      // Don't need to do anything...?

      return;
    } else if (message.kind == "change" && message.data != null) {
      // Pass data into the core
      subscription.current?.send(new Message(message.data));
    }
  };

  const initialized = useRef(false);

  // Initialize core and WASM
  useEffect(
    () => {
      if (!initialized.current) {
        initialized.current = true;

        (async () => {
          try {
            await wasmInitialized;

            // Initialize the Core with WASM after module is loaded
            core.current.initialize();

            // Subscribe to the BroadcastChannel
            channel.current.onmessage = onMessage;

            // Open the document
            core.current.update(eventOpen());

            // Ask all peers to reset
            let message: SyncMessage = {
              kind: "reset",
            };

            channel.current.postMessage(message);
          } catch (error) {
            console.error("Error during WASM initialization:", error);
          }
        })();

        const ch = channel.current;
        return () => {
          ch.onmessage = null;
        };
      }
    },
    /*once*/ [],
  );

  // Event handlers

  const onChange = ({ start, end, text }: ChangeEvent): void => {
    log(`onChange ${start} ${end} "${text}"`);

    core.current.update(eventReplace(BigInt(start), BigInt(end), text));
  };

  const onSelect = ({ start, end }: SelectEvent): void => {
    log(`onSelect ${start} ${end}`);

    let event =
      start == end
        ? eventMoveCursor(BigInt(end))
        : eventSelect(BigInt(start), BigInt(end));

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
        <main className="grow flex flex-col">
          <div className="grow basis-1 flex flex-col">
            <Textarea
              className="p-3 grow resize-none w-full focus:outline-none"
              selectionStart={selection.start}
              selectionEnd={selection.end}
              onSelect={onSelect}
              onChange={onChange}
              value={view.text}
            />
          </div>
          {LOG_EDITS ? (
            <div className="grow basis-1 overflow-scroll">
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
