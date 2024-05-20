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
  TimerOutput,
} from "shared_types/types/shared_types";
import {
  EffectVariantRender,
  ViewModel,
  Request,
  EffectVariantKeyValue,
  EffectVariantPubSub,
  EffectVariantTimer,
  PubSubOperationVariantPublish,
  PubSubOperationVariantSubscribe,
  TimerOperationVariantStart,
  TimerOutputVariantFinished,
  TimerOutputVariantCreated,
  TimerOperationVariantCancel,
  KeyValueOperationVariantGet,
  KeyValueOperationVariantSet,
  KeyValueResultVariantOk,
  KeyValueResponseVariantGet,
  KeyValueResponseVariantSet,
  ValueVariantNone,
  ValueVariantBytes,
} from "shared_types/types/shared_types";
import {
  BincodeSerializer,
  BincodeDeserializer,
} from "shared_types/bincode/mod";
import { Dispatch, MutableRefObject, SetStateAction } from "react";

type Response = Message | TimerOutput | KeyValueResult;

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
  channel: MutableRefObject<BroadcastChannel>;
  subscriptionId: MutableRefObject<number | null>;

  constructor(
    setState: Dispatch<SetStateAction<ViewModel>>,
    setTimers: Dispatch<SetStateAction<Timers>>,
    channel: MutableRefObject<BroadcastChannel>,
    subscriptionId: MutableRefObject<number | null>,
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

      case EffectVariantTimer: {
        const timerOp = (effect as EffectVariantTimer).value;

        switch (timerOp.constructor) {
          case TimerOperationVariantStart: {
            let { id: startId, millis } = timerOp as TimerOperationVariantStart;

            let handle = window.setTimeout(() => {
              // Drop the timer
              this.setTimers((ts) => {
                let { [Number(startId)]: _, ...rest } = ts;

                return rest;
              });

              this.respond(id, new TimerOutputVariantFinished(startId));
            }, Number(millis));
            this.setTimers((ts) => ({ [Number(startId)]: handle, ...ts }));

            this.respond(id, new TimerOutputVariantCreated(startId));

            break;
          }

          case TimerOperationVariantCancel: {
            let { id: cancelId } = timerOp as TimerOperationVariantCancel;

            this.setTimers((ts) => {
              let { [Number(cancelId)]: handle, ...rest } = ts;
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
            const bytes: number[] | null =
              data == null ? data : JSON.parse(data);
            const value =
              bytes == null
                ? new ValueVariantNone()
                : new ValueVariantBytes(bytes);

            console.log(`Loaded document (${bytes?.length || 0} bytes)`);
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
            // FIXME JSON is not exactly a space efficient format
            window.localStorage.setItem(writeKey, JSON.stringify(writeValue));

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
