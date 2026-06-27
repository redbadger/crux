import { CoreFFI } from "shared";
import type { Effect, Event } from "shared_types/app";

// WORKAROUND: automerge uses `features = ["wasm"]` in its Cargo.toml, which
// enables the `getrandom/js` feature. That compiles a wasm-bindgen import
// (`__wbg_getRandomValues_8aa3112c6615eef6`) into the WASM binary so that
// automerge can call `crypto.getRandomValues` to generate random actor IDs.
//
// boltffi deliberately stubs ALL `__wbindgen_placeholder__` imports as
// "Unimplemented" — it replaces wasm-bindgen interop entirely and doesn't
// provide browser API bindings. There is currently no way to inject custom
// `__wbindgen_placeholder__` implementations through the `instantiateBoltFFI`
// API, so we intercept `WebAssembly.instantiate` to replace the stub with the
// real `crypto.getRandomValues` before the module is loaded.
//
// The function name encodes a hash of its wasm-bindgen signature, so it could
// change if the `getrandom` version changes — grep for the new name in the
// WASM binary's imports if this breaks after a dependency update:
//   node -e "const fs=require('fs'); WebAssembly.compile(fs.readFileSync('generated/pkg/shared_bg.wasm')).then(m=>console.log(WebAssembly.Module.imports(m).map(i=>i.module+'::'+i.name).join('\n')))"
//
// The proper fix is either:
//   a) Remove `features = ["wasm"]` from the `automerge` dependency in
//      shared/Cargo.toml and configure `getrandom` with `features = ["custom"]`
//      plus a boltffi-compatible random implementation, or
//   b) Ask boltffi to natively provide `crypto.getRandomValues` for the
//      `__wbindgen_placeholder__` namespace (feature request to boltffi).
if (typeof WebAssembly !== "undefined" && typeof crypto !== "undefined") {
  const _origInstantiate = WebAssembly.instantiate.bind(WebAssembly);
  let _memory: WebAssembly.Memory | null = null;

  (WebAssembly as any).instantiate = (
    source: BufferSource | WebAssembly.Module,
    importObject?: WebAssembly.Imports,
  ) => {
    if (importObject?.["__wbindgen_placeholder__"]) {
      const stubs = importObject["__wbindgen_placeholder__"] as object;
      importObject["__wbindgen_placeholder__"] = new Proxy(stubs, {
        get(target, prop) {
          if (prop === "__wbg_getRandomValues_8aa3112c6615eef6") {
            return (ptr: number, len: number) => {
              crypto.getRandomValues(new Uint8Array(_memory!.buffer, ptr, len));
            };
          }
          return Reflect.get(target, prop);
        },
      }) as WebAssembly.ModuleImports;
    }
    return (
      _origInstantiate(
        source as any,
        importObject,
      ) as Promise<WebAssembly.WebAssemblyInstantiatedSource>
    ).then((result) => {
      _memory = result.instance.exports["memory"] as WebAssembly.Memory;
      return result;
    });
  };
}
import {
  ViewModel,
  Request,
  matchEffect,
  matchPubSubOperation,
  matchTimeRequest,
  matchKeyValueOperation,
  timeResponseDurationElapsed,
  keyValueResultOk,
  keyValueResponseGet,
  keyValueResponseSet,
  valueNone,
  valueBytes,
  serializeEvent,
  serializeKeyValueResult,
  serializeTimeResponse,
} from "shared_types/app";
import { BincodeSerializer, BincodeDeserializer } from "shared_types/bincode";
import type { Serializer } from "shared_types/serde";
import { Dispatch, RefObject, SetStateAction } from "react";

export type Timers = {
  [key: number]: number;
};

export type SyncMessage = {
  kind: "change" | "reset";
  data?: number[];
};

export class Core {
  core: CoreFFI | null = null;
  setState: Dispatch<SetStateAction<ViewModel>>;
  setTimers: Dispatch<SetStateAction<Timers>>;
  channel: RefObject<BroadcastChannel>;
  subscriptionId: RefObject<number | null>;

  constructor(
    setState: Dispatch<SetStateAction<ViewModel>>,
    setTimers: Dispatch<SetStateAction<Timers>>,
    channel: RefObject<BroadcastChannel>,
    subscriptionId: RefObject<number | null>,
  ) {
    // Don't initialize CoreFFI here - wait for WASM to be loaded
    this.setState = setState;
    this.setTimers = setTimers;
    this.channel = channel;
    this.subscriptionId = subscriptionId;
  }

  initialize() {
    if (!this.core) {
      this.core = CoreFFI.new();
    }
  }

  view(): ViewModel {
    if (!this.core) {
      throw new Error("Core not initialized. Call initialize() first.");
    }
    return deserializeView(this.core.view());
  }

  update(event: Event) {
    if (!this.core) {
      throw new Error("Core not initialized. Call initialize() first.");
    }
    console.log("event", event);

    const serializer = new BincodeSerializer();
    serializeEvent(event, serializer);

    const effects = this.core.update(serializer.getBytes());

    const requests = deserializeRequests(effects);
    for (const { id, effect } of requests) {
      this.processEffect(id, effect);
    }
  }

  private processEffect(id: number, effect: Effect) {
    console.log("effect", effect);

    matchEffect(effect, {
      Render: () => {
        this.setState(this.view());
      },

      PubSub: ({ value: pubSubOp }) => {
        matchPubSubOperation(pubSubOp, {
          Publish: (op) => {
            const message: SyncMessage = {
              kind: "change",
              data: op.value,
            };
            this.channel.current.postMessage(message);
          },
          Subscribe: () => {
            this.subscriptionId.current = id;
          },
        });
      },

      Time: ({ value: timerOp }) => {
        matchTimeRequest(timerOp, {
          Now: () => {},
          NotifyAt: () => {},
          NotifyAfter: (op) => {
            const { id: startId, duration } = op;
            const milliseconds = Number(duration.nanos) / 1e6;

            const handle = window.setTimeout(() => {
              // Drop the timer
              this.setTimers((ts) => {
                const { [Number(startId.value)]: _, ...rest } = ts;
                return rest;
              });

              this.respond(id, (s) =>
                serializeTimeResponse(timeResponseDurationElapsed(startId), s),
              );
            }, milliseconds);
            this.setTimers((ts) => ({
              [Number(startId.value)]: handle,
              ...ts,
            }));
          },
          Clear: (op) => {
            const cancelId = op.id;
            this.setTimers((ts) => {
              const { [Number(cancelId.value)]: handle, ...rest } = ts;
              window.clearTimeout(handle);
              return rest;
            });
          },
        });
      },

      KeyValue: ({ value: request }) => {
        matchKeyValueOperation(request, {
          Get: (op) => {
            const readKey = op.key;
            const data = window.localStorage.getItem(readKey);
            const bytes: number[] = data == null ? [] : JSON.parse(data);
            const value = bytes.length === 0 ? valueNone() : valueBytes(bytes);

            console.log(`Loaded document (${bytes.length} bytes)`);
            const result = keyValueResultOk(keyValueResponseGet(value));
            this.respond(id, (s) => serializeKeyValueResult(result, s));
          },
          Set: (op) => {
            const { key: writeKey, value: writeValue } = op;
            console.log(`Saving document (${writeValue.length} bytes)`);
            window.localStorage.setItem(
              writeKey,
              JSON.stringify(Array.from(writeValue)),
            );
            const result = keyValueResultOk(keyValueResponseSet(valueNone()));
            this.respond(id, (s) => serializeKeyValueResult(result, s));
          },
          Delete: () => {},
          Exists: () => {},
          ListKeys: () => {},
        });
      },
    });
  }

  respond(id: number, serialize: (s: Serializer) => void) {
    if (!this.core) {
      throw new Error("Core not initialized. Call initialize() first.");
    }
    const serializer = new BincodeSerializer();
    serialize(serializer);

    const effects = this.core.resolve(id, serializer.getBytes());
    const requests = deserializeRequests(effects);

    for (const { id, effect } of requests) {
      this.processEffect(id, effect);
    }
  }
}

function deserializeRequests(bytes: Uint8Array | number[]): Request[] {
  const deserializer = new BincodeDeserializer(asBytes(bytes));
  const len = deserializer.deserializeLen();
  const requests: Request[] = [];
  for (let i = 0; i < len; i++) {
    const request = Request.deserialize(deserializer);
    requests.push(request);
  }
  return requests;
}

function deserializeView(bytes: Uint8Array | number[]): ViewModel {
  return ViewModel.deserialize(new BincodeDeserializer(asBytes(bytes)));
}

function asBytes(bytes: Uint8Array | number[]): Uint8Array {
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
}
