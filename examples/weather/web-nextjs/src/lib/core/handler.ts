import type {
  ClearTimer,
  DeleteSecret,
  EffectHandler,
  FetchSecret,
  GetValue,
  HttpRequest,
  HttpResult,
  Location,
  NotifyAfter,
  SecretDeleteResponse,
  SecretFetchResponse,
  SecretStoreResponse,
  SetValue,
  StoreSecret,
  TimerId,
  ValueResult,
} from "shared_types/app";
import {
  createLocalStorageKeyValueHandler,
  fetchHttpHandler,
  TimeoutTimeHandler,
} from "shared_types/app";

import * as location from "./location";
import * as secret from "./secret";

/// The shell's side of the effect protocol.
///
/// `WeatherHandler` implements the generated `EffectHandler`: one method per
/// operation the app declares, each returning the single output that operation
/// is answered with. The generated `EffectDispatcher` does the resolving, so
/// nothing here decides when — or how often — to call `resolve`.
///
/// There is no `render` method: the generated `Core` intercepts `Render`
/// before the dispatcher sees it and calls the `onView` callback instead.
///
/// `fetchHttpHandler`, `createLocalStorageKeyValueHandler` and
/// `TimeoutTimeHandler` are what `crux_http`, `crux_kv` and `crux_time` ship,
/// generated into `shared_types/app` because the codegen binary asks for them.
/// The rules for mapping a `fetch` response, answering a store operation or
/// answering a timer belong to those crates, so the only thing this shell
/// writes for them is the line that delegates.
export class WeatherHandler implements EffectHandler {
  private readonly httpHandler = fetchHttpHandler;
  /// `localStorage` is shared with the whole origin, so the app's keys get a
  /// prefix of their own.
  private readonly keyValueHandler =
    createLocalStorageKeyValueHandler("weather.");
  /// The timer table is state, so there is one handler, for the page's life.
  private readonly timeHandler = new TimeoutTimeHandler();

  // ANCHOR: http
  http(operation: HttpRequest): Promise<HttpResult> {
    return this.httpHandler.request(operation);
  }
  // ANCHOR_END: http

  kvGet(operation: GetValue): Promise<ValueResult> {
    return this.keyValueHandler.get(operation);
  }

  kvSet(operation: SetValue): Promise<ValueResult> {
    return this.keyValueHandler.set(operation);
  }

  timeNotifyAfter(operation: NotifyAfter): Promise<TimerId> {
    return this.timeHandler.notifyAfter(operation);
  }

  timeClear(operation: ClearTimer): Promise<TimerId> {
    return this.timeHandler.clear(operation);
  }

  isLocationEnabled(): Promise<boolean> {
    return location.isLocationEnabled();
  }

  getLocation(): Promise<Location | null> {
    return location.getLocation();
  }

  fetchSecret(operation: FetchSecret): Promise<SecretFetchResponse> {
    return secret.fetch(operation);
  }

  storeSecret(operation: StoreSecret): Promise<SecretStoreResponse> {
    return secret.store(operation);
  }

  deleteSecret(operation: DeleteSecret): Promise<SecretDeleteResponse> {
    return secret.remove(operation);
  }
}
