import type { ViewModel } from "shared_types/app";
import { Core } from "shared_types/app";

import { LiveBridge } from "./bridge";
import { WeatherHandler } from "./handler";

export { LiveBridge } from "./bridge";
export { WeatherHandler } from "./handler";

// ANCHOR: core_base
/// Everything the shell has to write to run a Crux core: a `CoreBridge` over
/// the FFI, an `EffectHandler` for the app's operations, and a callback for
/// the view model. The generated `Core` owns the loop between them.
export function createCore(onView: (view: ViewModel) => void): Core {
  return new Core(new LiveBridge(), new WeatherHandler(), onView);
}
// ANCHOR_END: core_base
