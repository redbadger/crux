"use client";

import type { NextPage } from "next";
import Head from "next/head";
import { useEffect, useRef, useState } from "react";

import Navbar from "../components/Navbar/Navbar";
import Textarea, {
  ChangeEvent,
  SelectEvent,
} from "../components/Textarea/Textarea";

import { SyncMessage, createCore } from "./core";
import type { Core, EffectSink } from "shared_types/app";
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
    new ViewModel("", textCursorPosition(BigInt(0)), null),
  );
  // Until the core has produced its first view there is nothing to edit, and
  // edits would be dropped (`core.current` is still null).
  const [ready, setReady] = useState(false);

  // TODO the state and channel handling should probably get
  // packaged up as a custom hook or something

  // Set by the core's `subscribe` handler; every peer message becomes one
  // item on this sink.
  const subscription = useRef<EffectSink<Message> | null>(null);
  // Created once, in the effect: a `useRef` initialiser would open a new
  // channel on every render.
  const channel = useRef<BroadcastChannel | null>(null);
  const core = useRef<Core | null>(null);

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
            // `Core.create` waits for the WASM module before building the
            // generated bridge over it.
            const ch = new BroadcastChannel("crux-note");
            channel.current = ch;

            core.current = await createCore(
              (view) => {
                setView(view);
                setReady(true);
              },
              channel,
              subscription,
            );

            // Subscribe to the BroadcastChannel
            ch.onmessage = onMessage;

            // Open the document
            core.current.update(eventOpen());

            // Ask all peers to reset
            let message: SyncMessage = {
              kind: "reset",
            };

            ch.postMessage(message);
          } catch (error) {
            console.error("Error during WASM initialization:", error);
          }
        })();

        return () => {
          if (channel.current) channel.current.onmessage = null;
        };
      }
    },
    /*once*/ [],
  );

  // Event handlers

  const onChange = ({ start, end, text }: ChangeEvent): void => {
    if (!ready) return;
    log(`onChange ${start} ${end} "${text}"`);

    core.current?.update(eventReplace(BigInt(start), BigInt(end), text));
  };

  const onSelect = ({ start, end }: SelectEvent): void => {
    log(`onSelect ${start} ${end}`);

    let event =
      start == end
        ? eventMoveCursor(BigInt(end))
        : eventSelect(BigInt(start), BigInt(end));

    core.current?.update(event);
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
          {view.error ? (
            <div role="alert" className="p-3 bg-red-100 text-red-800">
              {view.error}
            </div>
          ) : null}
          {!ready ? (
            <div role="status" className="px-3 pt-2 text-sm text-slate-500">
              Loading…
            </div>
          ) : null}
          <div className="grow basis-1 flex flex-col">
            <Textarea
              className="p-3 grow resize-none w-full focus:outline-none"
              selectionStart={selection.start}
              selectionEnd={selection.end}
              onSelect={onSelect}
              onChange={onChange}
              value={view.text}
              disabled={!ready}
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
