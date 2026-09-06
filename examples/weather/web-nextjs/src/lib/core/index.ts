import type { Dispatch, SetStateAction } from "react";

import { CoreFfi } from "shared";
import type {
  Clear,
  Delete,
  EffectHandler,
  Event,
  Fetch,
  Get,
  HttpRequest,
  HttpResult,
  Location,
  NotifyAfter,
  SecretDeleteResponse,
  SecretFetchResponse,
  SecretStoreResponse,
  Set as SetValue,
  Store,
  TimerId,
  ValueResult,
  ViewModel,
} from "shared_types/app";
import {
  EffectDispatcher,
  Request,
  deserializeViewModel,
  serializeEvent,
} from "shared_types/app";
import { BincodeDeserializer, BincodeSerializer } from "shared_types/bincode";
import * as http from "./http";
import * as kv from "./kv";
import * as location from "./location";
import * as secret from "./secret";
import * as time from "./time";

// ANCHOR: core_base
/// The shell's side of the effect protocol.
///
/// `Core` implements the generated `EffectHandler`: one method per operation
/// the app declares, each returning the single output that operation is
/// answered with. The generated `EffectDispatcher` does the resolving, so
/// nothing here decides when — or how often — to call `resolve`.
export class Core implements EffectHandler {
  core: CoreFfi;
  callback: Dispatch<SetStateAction<ViewModel>>;
  private readonly dispatcher: EffectDispatcher;

  constructor(callback: Dispatch<SetStateAction<ViewModel>>) {
    this.callback = callback;
    this.core = CoreFfi.new();
    this.dispatcher = new EffectDispatcher(this, (id, bytes) =>
      this.respond(id, bytes),
    );
  }

  update(event: Event) {
    const serializer = new BincodeSerializer();
    serializeEvent(event, serializer);

    this.dispatch(this.core.update(serializer.getBytes()));
  }
  // ANCHOR_END: core_base

  render(): void {
    this.callback(deserializeView(this.core.view()));
  }

  // ANCHOR: http
  http(operation: HttpRequest): Promise<HttpResult> {
    return http.request(operation);
  }
  // ANCHOR_END: http

  kvGet(operation: Get): Promise<ValueResult> {
    return kv.get(operation);
  }

  kvSet(operation: SetValue): Promise<ValueResult> {
    return kv.set(operation);
  }

  timeNotifyAfter(operation: NotifyAfter): Promise<TimerId> {
    return time.notifyAfter(operation);
  }

  timeClear(operation: Clear): void {
    time.clear(operation);
  }

  isLocationEnabled(): Promise<boolean> {
    return location.isLocationEnabled();
  }

  getLocation(): Promise<Location | null> {
    return location.getLocation();
  }

  fetchSecret(operation: Fetch): Promise<SecretFetchResponse> {
    return secret.fetch(operation);
  }

  storeSecret(operation: Store): Promise<SecretStoreResponse> {
    return secret.store(operation);
  }

  deleteSecret(operation: Delete): Promise<SecretDeleteResponse> {
    return secret.remove(operation);
  }

  // ANCHOR: respond
  /// The dispatcher calls this with the serialized output of a handler
  /// method; the core answers with the next batch of requests.
  private respond(id: number, bytes: Uint8Array) {
    this.dispatch(this.core.resolve(id, bytes));
  }

  private dispatch(effects: Uint8Array | number[]) {
    for (const request of deserializeRequests(effects)) {
      this.dispatcher.dispatch(request);
    }
  }
  // ANCHOR_END: respond
}

function deserializeRequests(bytes: Uint8Array | number[]) {
  const deserializer = new BincodeDeserializer(asBytes(bytes));
  const len = deserializer.deserializeLen();
  const requests: Request[] = [];
  for (let i = 0; i < len; i++) {
    const request = Request.deserialize(deserializer);
    requests.push(request);
  }
  return requests;
}

function deserializeView(bytes: Uint8Array | number[]) {
  return deserializeViewModel(new BincodeDeserializer(asBytes(bytes)));
}

function asBytes(bytes: Uint8Array | number[]): Uint8Array {
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
}
