import type {
  Clear,
  Delete,
  EffectHandler,
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
} from "shared_types/app";

import * as http from "./http";
import * as kv from "./kv";
import * as location from "./location";
import * as secret from "./secret";
import * as time from "./time";

/// The shell's side of the effect protocol.
///
/// `WeatherHandler` implements the generated `EffectHandler`: one method per
/// operation the app declares, each returning the single output that operation
/// is answered with. The generated `EffectDispatcher` does the resolving, so
/// nothing here decides when — or how often — to call `resolve`.
///
/// There is no `render` method: the generated `Core` intercepts `Render`
/// before the dispatcher sees it and calls the `onView` callback instead.
export class WeatherHandler implements EffectHandler {
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
}
