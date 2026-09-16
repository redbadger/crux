import type { ViewModel } from "shared_types/app";
import { Core } from "shared_types/app";

import { WeatherHandler } from "./handler";

export { WeatherHandler } from "./handler";

// ANCHOR: core_base
/// Everything the shell has to write to run a Crux core: an `EffectHandler`
/// for the app's operations and a callback for the view model. `Core.create`
/// waits for the wasm module, builds the generated `FfiBridge` over it, and
/// owns the loop between them.
export function createCore(
  onView: (view: ViewModel) => void,
): Promise<Core> {
  return Core.create(new WeatherHandler(), onView);
}
// ANCHOR_END: core_base
