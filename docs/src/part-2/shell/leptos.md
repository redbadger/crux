# Leptos

In Leptos the shell and the core are both Rust, so there's no FFI boundary — `Effect` values flow directly, no bincode in between. The handshake is simpler than iOS or Android for that reason, but the render model is far enough from React or SwiftUI that it's worth pausing on before we get to the code.

## Components run once

In React, a component function runs every time state changes. Hooks — `useState`, `useEffect` — exist to keep values alive across those reruns and to schedule side-effects at the right moment.

Leptos doesn't work that way. A `#[component]` function runs **once**, when it mounts. Signals, context, and closures created inside the body are created once and stay. What reruns is the fine-grained parts inside the `view!` macro: a `move ||` closure tracks the signals it reads, and reruns *only that closure* when any of them change. The rest of the function body never runs again.

So this works without ceremony:

```rust,noplayground
let (view, set_view) = signal(core.view());
```

The signal is created on mount and persists for the life of the component. There's no `useState` equivalent because the closure *is* the persistence.

The practical consequence: the root component creates the core, the view signal, and the dispatcher once, and hands them out to the tree.

## Booting the core

`main.rs` is trivial — it mounts the root component:

```rust,noplayground
{{#include ../../../../examples/weather/web-leptos/src/main.rs:main}}
```

The setup lives in `lib.rs`:

```rust,noplayground
{{#include ../../../../examples/weather/web-leptos/src/lib.rs:app}}
```

Three things leave this function.

The **core** is wrapped in an `Rc` because the dispatcher and the startup effect each need their own handle. Both captures own a clone of the same `shared::Core<Weather>`.

The **view signal** `(view, set_view)` holds the current `ViewModel`. It's initialised from `core.view()` so the first render has something to paint. Leptos itself doesn't know what a Crux view model is — to Leptos, this is just a `Signal<ViewModel>` that gets `set` from somewhere.

The **dispatcher** is an `UnsyncCallback<Event>` — a thin wrapper around `Rc<dyn Fn>`. It takes an `Event`, pushes it into the core, and resolves whatever comes back. `provide_context` parks it on the component tree so every descendant can pull it without prop-threading.

The `Effect::new` at the bottom fires `Event::Start` once on mount. It has to be an effect rather than a bare function call because `provide_context` hasn't finished registering until the component body returns; deferring `Start` to the post-mount effect queue ensures startup doesn't race ahead of the tree being ready for it.

## The signal model

Two kinds of state cross between the shell and the core, and they use different mechanisms:

- **View model → shell:** a *signal*. Leptos reads it reactively.
- **Events → core:** a *callback*. The shell invokes it imperatively.

It's tempting to make both signals — a `(event, set_event)` pair that an `Effect` watches and forwards to the core. An earlier iteration of this shell did exactly that. It works, but it's wrong in two ways.

First, signals conflate consecutive writes. If you `set_event.set(A)` and then `set_event.set(B)` before the reactive system flushes, `A` is lost. Events can't be lost — they're commands, each one has to reach the core in order.

Second, signals model state, not commands. Using a callback for events and a signal for the view model keeps the directionality explicit: the shell *asks* the core to do something; the core *tells* the shell what its state is now.

The dispatcher lives in context:

```rust,noplayground
{{#include ../../../../examples/weather/web-leptos/src/components/mod.rs:dispatch}}
```

`UnsyncCallback` rather than `Callback` because `Rc<shared::Core<Weather>>` is `!Send` — WASM is single-threaded, so there's no thread boundary to worry about and no reason to pay for `Arc` / `Send` / `Sync`.

Child components pull it with `use_dispatch()` and fire events as:

```rust,noplayground
dispatch.run(Event::Active(ActiveEvent::ResetApiKey));
```

## Projecting the view model

The root holds a `Signal<ViewModel>`, but the individual screens only care about their own slice. A naive approach would clone the whole view model into each screen on every change; a better one is to *project* — derive a `Memo<SubViewModel>` per stage and hand each screen its own signal.

```rust,noplayground
{{#include ../../../../examples/weather/web-leptos/src/lib.rs:projections}}
```

A `Memo` is a derived signal with built-in equality checking — it only notifies downstream readers when the projected value actually differs from the last. So `home_vm` emits only when the top-level view model is in the `Home` variant *and* the inner `HomeViewModel` has changed.

The `_ => Default::default()` branch is what makes this type-check without forcing `Memo<Option<…>>` on every screen. The fallback is never rendered: each `<Show when=…>` below gates its subtree to the matching variant, so the `Default` value only exists to satisfy the type system.

This requires `Default` impls on the stage view models, which live in `shared/src/view/`:

```rust
{{#include ../../../../examples/weather/shared/src/view/onboard.rs:onboard_default}}
```

Enum `#[derive(Default)]` with a `#[default]` variant works for unit variants (like `LocalWeatherViewModel::CheckingPermission`); struct variants need a manual impl.

## Reading the projection

The screen takes its projected signal as a prop and reads fields inside reactive closures. The `Home` screen is representative:

```rust,noplayground
{{#include ../../../../examples/weather/web-leptos/src/components/home.rs:home_view}}
```

Two read patterns appear here:

- **`.read()`** returns a guard that derefs to `&HomeViewModel`. It's the direct form — `vm.read().local_weather.clone()` reads a field and clones the inner value. Good for single-field access.
- **`.with(|v| …)`** takes a closure that borrows the whole value. Good when you want to project or test — `vm.with(|v| v.favorites.clone())` here, or `view.with(|v| matches!(v, ViewModel::Loading))` for a discriminant check.

Both avoid cloning the outer view model. `.get()` is available too but clones the entire value — fine for a `Signal<bool>` or `Signal<String>`, wasteful for a `Signal<HomeViewModel>`.

Each `move ||` closure tracks only the fields it reads. When `local_weather` changes but `favorites` doesn't, only the first closure reruns. That's the granularity Leptos gives you — no VDOM diffing, just precise subscriptions.

## Handling effects

`core/mod.rs` owns the per-effect dispatch. The kernel is a `match` on the `Effect` enum:

```rust,noplayground
{{#include ../../../../examples/weather/web-leptos/src/core/mod.rs:process_effect}}
```

Five capability branches plus `Render`, which writes the current view model into the signal. The shell and the core share the same Rust types, so the match compiles into a direct call — no serialisation layer between them.

Each capability lives in its own file. Here's HTTP:

```rust,noplayground
{{#include ../../../../examples/weather/web-leptos/src/core/http.rs:resolve}}
```

`spawn_local` because WASM is single-threaded — there's no runtime to multiplex on. The closure makes the fetch call, then hands the response back to the core via `resolve_effect`:

```rust,noplayground
{{#include ../../../../examples/weather/web-leptos/src/core/mod.rs:core_base}}
```

`core.resolve(...)` returns a **fresh batch of effects**, so `resolve_effect` loops back through `process_effect`. A Crux command with `.await` points produces its next effect only after the previous one resolves, so the shell has to keep going until the command's task actually finishes.

The other capabilities — `kv`, `location`, `secret`, `time` — follow the same shape: take the request, do the work, resolve, recurse.

## Shared components

Screens compose the shared components in `components/common/` — `Card`, `Button`, `Spinner`, `TextField`, `StatusMessage`, `Modal`, and so on. They're plain Tailwind-styled Leptos components that know nothing about Crux; screens assemble them and wire each button to the dispatcher.

A `Button` takes `label: Signal<String>` (so static strings and reactive closures both work), `enabled: Signal<bool>`, an optional `icon: IconData` (`&'static IconWeightData` — phosphor's own alias), and an `on_click: UnsyncCallback<()>`. In practice the call site reads:

```rust,noplayground
<Button
    label="Reset API Key"
    icon=KEY
    variant=ButtonVariant::Secondary
    on_click=UnsyncCallback::new(move |()| {
        dispatch.run(Event::Active(ActiveEvent::ResetApiKey));
    })
/>
```

The `UnsyncCallback::new(move |()| dispatch.run(…))` pattern is the same bridge as elsewhere: Leptos's imperative event world meets the core's event log.

## What's next

That's the Leptos shell end-to-end. The structural story is the same as iOS and Android — events in, effects out, view model drives the tree — but the reactivity primitives are Leptos-specific: a signal for the view model, Memos for the per-stage projection, a callback for events, and a `move ||` per reactive slot.

Happy building!
