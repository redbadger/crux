import {
  process_event,
  handle_response,
  view as coreView,
} from "shared/shared";
import type {
  Effect,
  Event,
  KeyValueResult,
  Message,
  TimeResponse,
} from "shared_types/types/shared_types";
import {
  EffectVariantRender,
  ViewModel,
  Request,
  EffectVariantKeyValue,
  EffectVariantPubSub,
  EffectVariantTime,
  PubSubOperationVariantPublish,
  PubSubOperationVariantSubscribe,
  KeyValueOperationVariantGet,
  KeyValueOperationVariantSet,
  KeyValueResultVariantOk,
  KeyValueResponseVariantGet,
  KeyValueResponseVariantSet,
  ValueVariantNone,
  ValueVariantBytes,
  TimeRequestVariantnotifyAfter,
  TimeResponseVariantdurationElapsed,
  TimeRequestVariantclear,
} from "shared_types/types/shared_types";
import {
  BincodeSerializer,
  BincodeDeserializer,
} from "shared_types/bincode/mod";
import { Dispatch, RefObject, SetStateAction } from "react";

type Response = Message | TimeResponse | KeyValueResult;

export type Timers = {
  [key: number]: number;
};

export type SyncMessage = {
  kind: "change" | "reset";
  data?: number[];
};

export class Core {
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
    this.setState = setState;
    this.setTimers = setTimers;
    this.channel = channel;
    this.subscriptionId = subscriptionId;
  }

  view(): ViewModel {
    return deserializeView(coreView());
  }

  update(event: Event) {
    console.log("event", event);

    const serializer = new BincodeSerializer();
    event.serialize(serializer);

    const effects = process_event(serializer.getBytes());

    const requests = deserializeRequests(effects);
    for (const { id, effect } of requests) {
      this.processEffect(id, effect);
    }
  }

  private processEffect(id: number, effect: Effect) {
    console.log("effect", effect);

    switch (effect.constructor) {
      case EffectVariantRender: {
        this.setState(deserializeView(coreView()));
        break;
      }

      case EffectVariantPubSub: {
        const pubSubOp = (effect as EffectVariantPubSub).value;

        switch (pubSubOp.constructor) {
          case PubSubOperationVariantPublish:
            let publish = pubSubOp as PubSubOperationVariantPublish;
            let message: SyncMessage = {
              kind: "change",
              data: publish.value,
            };

            this.channel.current.postMessage(message);

            break;
          case PubSubOperationVariantSubscribe:
            this.subscriptionId.current = id;

            break;
        }
        break;
      }

      case EffectVariantTime: {
        const timerOp = (effect as EffectVariantTime).value;

        switch (timerOp.constructor) {
          case TimeRequestVariantnotifyAfter: {
            let { id: startId, duration } =
              timerOp as TimeRequestVariantnotifyAfter;
            let millis = Number(duration.nanos) / 1e6;

            let handle = window.setTimeout(() => {
              // Drop the timer
              this.setTimers((ts) => {
                let { [Number(startId)]: _, ...rest } = ts;

                return rest;
              });

              this.respond(id, new TimeResponseVariantdurationElapsed(startId));
            }, millis);
            this.setTimers((ts) => ({ [Number(startId)]: handle, ...ts }));

            break;
          }

          case TimeRequestVariantclear: {
            let { id: cancelId } = timerOp as TimeRequestVariantclear;

            this.setTimers((ts) => {
              let { [Number(cancelId.value)]: handle, ...rest } = ts;
              window.clearTimeout(handle);

              return rest;
            });
          }
        }
        break;
      }

      case EffectVariantKeyValue: {
        const request = (effect as EffectVariantKeyValue).value;
        switch (request.constructor) {
          case KeyValueOperationVariantGet: {
            const { key: readKey } = request as KeyValueOperationVariantGet;

            const data = window.localStorage.getItem(readKey);
            const bytes =
              data == null
                ? new Uint8Array()
                : new Uint8Array(JSON.parse(data));
            const value =
              bytes.length === 0
                ? new ValueVariantNone()
                : new ValueVariantBytes(bytes);

            console.log(`Loaded document (${bytes.length} bytes)`);
            this.respond(
              id,
              new KeyValueResultVariantOk(
                new KeyValueResponseVariantGet(value),
              ),
            );

            break;
          }

          case KeyValueOperationVariantSet: {
            const { key: writeKey, value: writeValue } =
              request as KeyValueOperationVariantSet;

            console.log(`Saving document (${writeValue.length} bytes)`);
            window.localStorage.setItem(
              writeKey,
              JSON.stringify(Array.from(writeValue)),
            );

            this.respond(
              id,
              new KeyValueResultVariantOk(
                new KeyValueResponseVariantSet(new ValueVariantNone()),
              ),
            );

            break;
          }
        }
        break;
      }
    }
  }

  respond(id: number, response: Response) {
    const serializer = new BincodeSerializer();
    response.serialize(serializer);

    const effects = handle_response(id, serializer.getBytes());
    const requests = deserializeRequests(effects);

    for (const { id, effect } of requests) {
      this.processEffect(id, effect);
    }
  }
}

function deserializeRequests(bytes: Uint8Array): Request[] {
  const deserializer = new BincodeDeserializer(bytes);
  const len = deserializer.deserializeLen();
  const requests: Request[] = [];
  for (let i = 0; i < len; i++) {
    const request = Request.deserialize(deserializer);
    requests.push(request);
  }
  return requests;
}

function deserializeView(bytes: Uint8Array): ViewModel {
  return ViewModel.deserialize(new BincodeDeserializer(bytes));
}
