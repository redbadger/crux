# React (Next.js)

The Next.js shell is TypeScript talking to a WASM blob. Events serialise as bincode, cross the FFI, return effect requests; the shell handles each one, serialises the response, sends it back — the same `Request`/`Response` loop as iOS and Android, just in JavaScript. The interesting half of the chapter is how React's render model shapes the shell, because it's the opposite of Leptos's.

## Components re-run on every render

In Leptos, a `#[component]` function runs *once*, at mount. Signals keep state alive across time; `move ||` closures inside `view!` re-render fine-grained slots.

React is the other way round. A function component runs **every time its state changes** — top to bottom, from scratch. Each render produces a new virtual DOM; React diffs against the previous one and patches the actual DOM. Hooks — `useState`, `useRef`, `useContext`, `useMemo`, `useCallback` — exist to keep values alive across those reruns and to schedule work at specific moments rather than on every render.

Three consequences for the Crux shell:

- The `Core` instance has to live in a `useRef`, not a plain local. A fresh `new Core()` on every render would break effect resolution mid-flight.
- The view model becomes `useState<ViewModel>`: React-owned state, whose setter triggers a re-render when the core hands us a new view model.
- The dispatcher is wrapped in `useCallback` so its reference is stable across renders. Button handlers that capture it don't then capture a moving target.

## Booting the Core

The root of the tree is a `CoreProvider`:

```typescript
{{#include ../../../../examples/weather/web-nextjs/src/lib/core/provider.tsx:provider}}
```

Three things happen in this component.

The `useRef` holds the `Core` across renders. `coreRef.current` points at the same instance every time the function runs; assigning once inside the init effect locks it in.

The init `useEffect` has an empty dep array, which React reads as "run on mount, once". It awaits the WASM module's `initialized` promise, then calls `createCore` and fires `Event::Start` to kick off the lifecycle. The `initialized.current` guard is belt-and-braces for StrictMode (on by default in Next.js), which double-invokes effects in development to surface resource-leak bugs.

The `dispatch` callback is wrapped in `useCallback(_, [])` so its reference is stable. Consumers of `useDispatch()` get the same function every render, which matters when passing it into handlers — otherwise every view update would invalidate every handler and trigger spurious re-renders of memoised children.

## The signal model — React edition

Same directional story as Leptos: state flows in, events flow out. The mechanisms are different, but the shape is the same. Two separate contexts:

```typescript
{{#include ../../../../examples/weather/web-nextjs/src/lib/core/provider.tsx:context}}
```

Splitting them matters. With both `view` and `dispatch` in one context, every `view` change re-renders every consumer of either side — even a component that only needed `dispatch`. Two contexts means `useDispatch()` consumers only re-render when the stable callback reference changes, which it never does.

Consumers pull either side with a hook:

```typescript
{{#include ../../../../examples/weather/web-nextjs/src/lib/core/provider.tsx:hooks}}
```

Then components fire events with:

```typescript
const dispatch = useDispatch();
dispatch(eventActive(activeEventResetApiKey()));
```

Both directions cross the FFI as bincode. `dispatch` is just a JS callback around the generated `Core`'s `update`; `setView` is a React state setter, handed to `Core` as its `onView` callback, which `Core` calls with the freshly deserialised view model after every `Effect::Render`.

## Projecting with useMemo

The root reads the whole `ViewModel` and picks off per-stage slices for each screen:

```typescript
{{#include ../../../../examples/weather/web-nextjs/src/app/page.tsx:app}}
```

`useMemo(() => …, [view])` is the React analogue of Leptos's `Memo` in intent: keep the projection logic explicit and rerun it when `view` changes. Coarser in one important way — React compares deps by reference, not value. Every `Effect::Render` produces a freshly deserialised `ViewModel`, so `view` is always a new object and the memo always recomputes. For Weather that's fine; the projection functions are cheap.

So the win here is mostly clarity: the stage-picking logic lives in one place, and each child receives the slice it cares about. It is not fine-grained reactivity, and it doesn't by itself make child handlers stable or suppress rerenders deeper in the tree. If you wanted that, you'd reach for `React.memo` and/or stable callbacks at the relevant component boundary — but this example doesn't need the extra machinery.

## Handling effects

The shell writes two small things and the generated `Core` owns the loop between them:

```typescript
{{#include ../../../../examples/weather/web-nextjs/src/lib/core/index.ts:core_base}}
```

`Core` serialises an event with `BincodeSerializer`, calls the bridge's `update`, deserialises the returned bytes into `Request` objects, handles `Render` itself (re-read the view, call `onView`), and hands everything else to the generated `EffectDispatcher`. There is no `switch` over the effect union anywhere: the dispatcher calls the method for the variant it received and resolves the request with what that method returns. When it does, `Core` calls the bridge's `resolve` and loops through any **new** effect requests that come back — a Crux command with `.await` points produces its next effect only after the previous one resolves, so the loop has to keep going until the command's task actually finishes, and now nothing in the shell has to remember to. Its public surface:

```typescript
export class Core {
    view: ViewModel;
    constructor(bridge: CoreBridge, handler: EffectHandler,
                onView: (view: ViewModel) => void);
    update(event: Event): void;
    process(requests: Request[]): void;
    processBytes(bytes: Uint8Array): void;
}
```

The bridge is the generated `CoreBridge` interface over the WASM export:

```typescript
{{#include ../../../../examples/weather/web-nextjs/src/lib/core/bridge.ts}}
```

Bytes in, bytes out. `CoreFfi.new()` touches the WASM module, so a `LiveBridge` can only be constructed once `initialized` has resolved — which is why `CoreProvider` builds the core inside its init effect rather than in the `useRef` initialiser.

The handler is `WeatherHandler`, one method per operation. HTTP looks like this:

```typescript
{{#include ../../../../examples/weather/web-nextjs/src/lib/core/handler.ts:http}}
```

One method, returning a `Promise<HttpResult>` — the operation declares that it is answered exactly once with an `HttpResult`, so that's the signature, and the dispatcher awaits it and resolves. The handler in `http.ts` is a `fetch` wrapper that turns the shared `HttpRequest` into a browser `Request` and the `Response` back into the shared `HttpResult`. `timeClear(operation: Clear): void` is the one notification here — nothing to return, nothing resolved. There is no `render` method at all: `Core` owns it, and the generated interface makes it optional.

The other capabilities — `kv`, `location`, `secret`, `time` — follow the same shape: one method per operation, returning that operation's output.

```admonish note title="Two TypeScript details"
`crux_kv`'s `Set` operation generates a type called `Set`, which shadows the
built-in — `handler.ts` imports it as `Set as SetValue`. And the generated union
already uses `kind` as its discriminant, so the request-kind accessor is a free
function, `effectRequestKind(effect)`, rather than a property.
```

The notes example goes one step further and uses a **stream**. Its `Subscribe`
operation is resolved once per message from a peer, so the generated handler
method takes a sink instead of returning a value:

```typescript
subscribe(_operation: Subscribe, sink: EffectSink<Message>): void {
  // Every message a peer broadcasts becomes one item on this sink, for as
  // long as the page lives.
  this.subscription.current = sink;
}
```

Its `NotesHandler` parks the sink and the page calls `sink.send(new Message(bytes))`
whenever a message arrives; each `send` resolves the original request again. Before the
handler API, that shell had to keep the request id in a ref and remember to
resolve it repeatedly and never terminate it — the sink is the same thing with
the bookkeeping generated.

## Shared components

Screens compose a set of Tailwind-styled presentational components in `src/app/components/common/`: `Card`, `Button`, `IconButton`, `Spinner`, `StatusMessage`, `TextField`, `ScreenHeader`, `SectionTitle`, `Modal`. Same names and variant set as the Leptos shell — `Button` takes `primary | secondary | danger`; `StatusMessage` takes `neutral | error` — so a reader who's seen both shells sees the same vocabulary twice in slightly different dialects. `clsx` handles the conditional-class plumbing.

The `Home` screen pulls its slice from props, calls `useDispatch` once, and wires buttons to events:

```typescript
{{#include ../../../../examples/weather/web-nextjs/src/app/components/HomeView.tsx:home_view}}
```

Icons come from `@phosphor-icons/react` as typed components (`<Key size={18} />`). The `icon` prop on `Button` takes a phosphor component directly; inside the component it's destructured as `{ icon: Icon }` so JSX can render it with a PascalCase tag.

## What's next

That's the Next.js shell. Structurally the same as the other shells — events in, effects out, view model drives the tree. What's distinctive is the render model (top-to-bottom on every state change, hooks as the persistence mechanism) and the two-context split that keeps dispatch-only consumers off the re-render path.

Happy building!
