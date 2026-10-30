import type {
  Clear,
  DeleteSecret,
  EffectHandler,
  FetchSecret,
  Get,
  HttpRequest,
  HttpResult,
  Location,
  NotifyAfter,
  SecretDeleteResponse,
  SecretFetchResponse,
  SecretStoreResponse,
  Set as SetValue,
  StoreSecret,
  TimerId,
  ValueResult,
} from "shared_types/app";
import { createFetchHttpHandler, TimeoutTimeHandler } from "shared_types/app";

import { createLocalStorageKeyValueHandler } from "./kv";
import * as location from "./location";
import * as secret from "./secret";

/**
 * The shell's side of the effect protocol.
 *
 * `WeatherHandler` implements the generated `EffectHandler`: one method per
 * operation the app declares, each returning the single output that operation
 * is answered with. The generated `EffectDispatcher` does the resolving, so
 * nothing here decides when — or how often — to call `resolve`.
 *
 * There is no `render` method: the generated `Core` intercepts `Render`
 * before the dispatcher sees it and calls the `onView` callback instead.
 */
export class WeatherHandler implements EffectHandler {
  private readonly httpHandler = createFetchHttpHandler();
  /** `localStorage` is shared with the whole origin, so the keys get a prefix. */
  private readonly keyValueHandler =
    createLocalStorageKeyValueHandler("weather.");
  private readonly timeHandler = new TimeoutTimeHandler();

  // ANCHOR: http
  http(operation: HttpRequest): Promise<HttpResult> {
    return this.httpHandler.request(operation);
  }
  // ANCHOR_END: http

  kvGet(operation: Get): Promise<ValueResult> {
    return this.keyValueHandler.get(operation);
  }

  kvSet(operation: SetValue): Promise<ValueResult> {
    return this.keyValueHandler.set(operation);
  }

  timeNotifyAfter(operation: NotifyAfter): Promise<TimerId> {
    return this.timeHandler.notifyAfter(operation);
  }

  timeClear(operation: Clear): Promise<TimerId> {
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
