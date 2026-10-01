import type { EffectHandler, ViewModel } from "shared_types/app";
import { Core } from "shared_types/app";

/// The shell's side of the effect protocol.
///
/// `CounterHandler` implements the generated `EffectHandler`, which has one
/// method per operation the app declares. The counter's only effect is
/// `Render`, and the generated `Core` handles that itself (reading the new
/// view model and passing it to the `onView` callback), so there is nothing
/// left for the handler to do.
export class CounterHandler implements EffectHandler {}

/// Everything the shell has to write to run a Crux core: an `EffectHandler`
/// for the app's operations and a callback for the view model. `Core.create`
/// waits for the wasm module, builds the generated `FfiBridge` over it, and
/// owns the loop between them.
export function createCore(onView: (view: ViewModel) => void): Promise<Core> {
  return Core.create(new CounterHandler(), onView);
}
