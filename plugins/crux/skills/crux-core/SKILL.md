---
name: crux-core
description: Best practices for the shared Rust core of a Crux app (crates `crux_core`, `crux_http`, `crux_kv`, `crux_time`). Load it before you write, change, review or test core code. This includes the `App` trait, `Event`, `Model`, `ViewModel`, the `#[effect]` enum, `Command` and command builders, capabilities and `#[derive(Operation)]` operations, nested state machines, the effect router and effect middleware, and core unit tests with `EffectTestExt` (`expect_*`, `resolve_*`). It describes the latest Crux patterns on `master`.
---

# Crux shared core: best practices

This skill tells you how to build the shared core of a Crux app. Most rules come from the
Crux book and the `weather`, `counter`, `counter-routing` and `notes` examples in
<https://github.com/redbadger/crux>. Section 5.2 is the exception. It is recommended
practice that no Crux example shows yet.

The rules follow the latest patterns on the Crux `master` branch. Some of these
patterns are not in a published release yet. Examples are `#[derive(Operation)]`,
one operation type for each `Effect` variant, and operation kinds. If a project
compiles against an older release, tell the user which rule needs a newer Crux. Do not
silently write the older pattern.

Sources to read when you need more detail:

- Book (master): <https://redbadger.github.io/crux/latest_master/>
- API docs (master): <https://redbadger.github.io/crux/latest_master/master_api_docs/crux_core/>
- Weather example, the reference for a real app:
  <https://github.com/redbadger/crux/tree/master/examples/weather/shared/src>

## Terms

Use these terms with these meanings only.

| Term | Meaning |
| --- | --- |
| core | The Rust crate that holds the behaviour. It has no side-effects. |
| shell | The native code (Swift, Kotlin, TypeScript, Rust) that draws the UI and runs effects. |
| event | A value that drives `update`. The shell sends some events. Commands send the others. |
| model | The internal state. It never crosses the FFI boundary. |
| view model | The data that the shell renders. `view` makes it from the model. |
| operation | The data for one kind of request to the shell. It has one output type and one kind. |
| effect | A variant of the `Effect` enum. It wraps one operation. |
| command | A `Command<Effect, Event>`. It is a recipe that emits effects and events. |
| capability | A protocol (operation types) plus a developer API (command builders). |

## 1. The core is pure

- **Do not do I/O in the core.** No network, file, clock, random number or log
  side-effect that the app depends on. Ask the shell for it with an effect. Pure code is
  deterministic, so you can test it without fakes.
- **Put orchestration in the core and execution in the shell.** The core decides what to
  fetch, in which order, and what to do with the result. The shell only performs each
  operation.
- **Do not use Tokio, async-std or other runtime-specific futures in a command.** The
  Crux executor runs only when the shell calls the core. A foreign future resolves late
  or never. If you must use a crate that needs a runtime, handle that effect in Rust
  outside the core, with the effect router or middleware (see section 7).
- **Do not give a command access to the model.** Commands run asynchronously and can run
  concurrently. Send the result back as an event and change the model in `update`.

## 2. The `App` and its types

Keep the `App` implementation thin. Put the logic on the model.

```rust
#[derive(Default)]
pub struct Weather;

impl App for Weather {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Self::Event, model: &mut Self::Model) -> Command<Effect, Event> {
        model.update(event)
    }

    fn view(&self, model: &Model) -> ViewModel {
        model.into()
    }
}
```

### Events

- **Treat `Event` as the public API of the core.** Name each variant for what happened
  (`LocationFetched`, `Retry`), not for what the core does next.
- **Give the shell one start event,** for example `Event::Start`. The core does nothing
  until the shell sends it. Start the first effects from this event, not from `Default`.
- **Namespace events by stage, screen or feature.** Wrap child events in a parent variant
  (`Event::Active(ActiveEvent)` or `Event::Power(power::Event)`). Section 5 tells you which
  of the two structures to use.
- **If a shell uses a bridge or type generation, mark internal events with both
  `#[serde(skip)]` and `#[facet(skip)]`.** Internal events carry effect results, and the
  shell never sends them. The bridge must not deserialize them, and type generation must
  not emit them. If a skipped variant holds a type without `Facet` (for example
  `crux_http::Result<...>`), mark that field `#[facet(opaque)]`. If only a Rust shell
  calls `Core` directly, you do not need these attributes.
- **Box large payloads** in event variants (`WeatherFetched(Box<Result<..>>)`) to keep
  the enum small.
- **Do not call `update` recursively.** To continue with another event, return
  `Command::event(next)`. Crux then sees both updates.

### Model

- **Model the app as state machines.** Use an enum with one variant for each mutually
  exclusive state. Each variant holds only the data for that state. Do not use a struct
  with many `Option` fields. Invalid states then cannot be represented.
- **The model does not cross the FFI boundary.** Do not derive `Serialize`, `Facet` or
  `Clone` on it unless a test or a feature needs them.
- **Wrap domain values in newtypes** (`ApiKey(String)`) where a bare `String` could mix
  up two meanings.

### View model

- **Make the view model exactly what the UI needs,** and nothing more. Format values,
  flatten nested state machines, and make user-facing strings in the core. The shell
  then contains no logic to test.
- **Shape the top-level view model like the lifecycle** (`Loading`, `Onboard(..)`,
  `Active(..)`, `Failed { message }`).
- **Implement `From<&Model> for ViewModel`** and call it from `view`. Keep `view` a pure
  projection of the model with no side-effects.

### Types that cross the FFI boundary

If a shell uses a bridge or type generation, apply these rules. The bridge serializes
these types, and type generation emits them for the shell. If only a Rust shell calls
`Core` directly, use `#[effect]` with no argument. You then need none of these derives,
unless the shell serializes a type itself.

- **Derive `Facet`, `Serialize` and `Deserialize`** on `Event`, `ViewModel`, operations,
  outputs and every type that they contain.
- **Give every enum that crosses the boundary an explicit representation.** Facet
  requires one. The Crux examples use `#[repr(C)]`.
- **Use `#[effect(facet_typegen)]` on the `Effect` enum when a shell uses a bridge.** The
  macro generates the `EffectFFI` impl that the bridge needs, and it registers each
  operation for type generation. It also makes each operation `Serialize` and
  `Deserialize`, and each output `Deserialize`.
- **Forward the `facet_typegen` feature in the shared crate,** and make
  `bin/codegen.rs` require it. Without this feature, the generated `EffectFfi` enum
  does not derive `Facet`, and type generation fails. The weather example does this:

  ```toml
  [[bin]]
  name = "codegen"
  required-features = ["codegen"]

  [features]
  codegen = ["facet_typegen"]  # plus the dependencies of the codegen binary
  facet_typegen = ["crux_core/facet_typegen"]
  ```

## 3. Effects and capabilities

### The `Effect` enum

- **Give each operation its own variant.** Name the variant for the operation, not for
  the capability (`KvGet(kv::GetValue)`, `KvSet(kv::SetValue)`, not `KeyValue(..)`).
- **List only the operations that the app uses.** The shell then implements only those
  operations.
- **Write a doc comment on each variant** that says why the app uses it.
- **Write a doc comment on each operation type** that states the contract for the shell.
  Type generation copies it onto the matching `EffectHandler` method.

```rust
#[effect(facet_typegen)]
pub enum Effect {
    /// Ask the shell to re-read the view model and repaint.
    Render(RenderOperation),
    /// Perform an HTTP request.
    Http(HttpRequest),
    /// Read the favourites list from the key-value store.
    KvGet(kv::GetValue),
    /// Write the favourites list to the key-value store.
    KvSet(kv::SetValue),
    /// Ask whether location services are enabled.
    IsLocationEnabled(IsLocationEnabled),
    /// Ask for the device's coordinates.
    GetLocation(GetLocation),
}
```

### Operations

- **Give each operation exactly one output type and one kind.** Do not make one
  operation enum with a response enum that has a variant for each request. With one
  output for each operation, the shell cannot send the wrong answer, and you need no
  `unreachable!()`.
- **Declare both with `#[derive(Operation)]`.** The kind tells the shell how many times
  to resolve the request:

  | Kind | Shell resolves | Builder | Output |
  | --- | --- | --- | --- |
  | `notify` | never | `Command::notify_shell` | `()` |
  | `request` | exactly once | `Command::request_from_shell` | one value |
  | `stream` | zero or more times | `Command::stream_from_shell` | one value for each item |

  ```rust
  #[derive(Operation, Facet, Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
  #[operation(request, output = Option<Location>)]
  pub struct GetLocation;
  ```

- **Run `cargo build`, `cargo test` or `cargo clippy --all-targets` after you change a
  kind.** A wrong builder for a kind fails there, but not in `cargo check` or the editor.
- **Do not use `std::result::Result` as an output.** Type generation cannot emit it.
  Use a two-variant enum in the style of `HttpResult` (`Ok(..)`, `Err(..)`) with
  `#[repr(C)]`. Give it a `From` impl, so that the builder can return a `Result` to app
  code.
- **Put the capability name in each app-defined operation name** (`FetchSecret`, not
  `Fetch`). Type generation puts all operations in one namespace on the shell side.

### Capabilities

- **Use the published capabilities first:** the `crux_core::render` module
  (`RenderOperation`, `render()`), `crux_http`, `crux_kv` and `crux_time`. An app needs
  about seven capabilities.
- **Do not use the deprecated capability APIs.** Use `crux_kv::store::KeyValue`, not
  `crux_kv::KeyValue`. Use `crux_time::clock::Time`, not `crux_time::Time`. Use
  `crux_http::Http`, not `crux_http::command::Http`.
- **Write a custom capability when the behaviour differs on each platform** (location,
  secure storage, biometrics). A capability is two things: the operation types, and a
  small `command` module of builder functions.
- **Return a command builder, not a `Command`,** from a capability function. The caller
  then binds it to its own event with `.then_send(..)`, or awaits it in async code.
- **Make builders generic over `Effect` and `Event`,** and bound them on each operation:

  ```rust
  #[must_use]
  pub fn get_location<Effect, Event>()
  -> RequestBuilder<Effect, Event, impl Future<Output = Option<Location>>>
  where
      Effect: Send + From<Request<GetLocation>> + 'static,
      Event: Send + 'static,
  {
      Command::request_from_shell(GetLocation)
  }
  ```

- **Wrap a remote API in a typed client module** (for example
  `effects/http/weather`). It builds the request, sets `.expect_json::<T>()`, and uses
  `.map(..)` to turn the HTTP result into a domain `Result<T, DomainError>`. Callers
  then never see HTTP details.

### HTTP rejections

- **Read a 4xx or 5xx response on the `Err` side.** `crux_http` turns a rejection into
  `Err(HttpError::Http { .. })`. An `Ok(Response)` never has a 4xx or 5xx status.
  Do not check `response.status()` for errors in the `Ok` arm, because that branch is
  dead code.
- **Get rejection details from the accessors:** `code()`, `body()`,
  `body_json::<T>()`, `header(name)`, `content_type()` and `headers()`. Match
  `HttpError::Http { code, .. }` with `..`, because the variant is `#[non_exhaustive]`.

## 4. Commands

- **Keep an orchestration in one command, unless a step needs the model.** A command
  can chain, combine and race effects, and it can carry data from one step to the next.
  Do not send an internal event back through `update` only to start the next effect.
  Commands cannot currently read or change the model. If a step must read or change the
  model, send an event, and start the next command from `update`.
- **Return exactly one `Command` from each `update` call.** Return `Command::done()`
  when there is nothing to do. Call `render()` when the view model changes.
- **Combine commands for concurrent work** with `Command::all([..])` or `a.and(b)`.
- **Sequence dependent work** with `.then_request(..)` and `.then_stream(..)` on a
  builder, or with `first.then(second)` on commands. `then` waits for `first` and its
  tasks to finish. It does not wait for the app to process the events of `first`.
- **Transform outputs with `.map(..)` on a builder,** before `.then_send(..)`.
- **Use `Command::new(|ctx| async move { .. })` only when the combinators are not
  enough.** In the task, use `ctx.request_from_shell`, `ctx.send_event` and
  `ctx.spawn`, or turn a builder into a future with `.into_future(ctx)`.
- **Lift child commands** to the parent event type with `.map_event(ParentEvent::Child)`
  in nested state machines. Features over one model return the root
  `Command<Effect, Event>` and need no `map_event` (see section 5).
- **Store an `AbortHandle` to cancel work.** Call `cmd.abort_handle()`, keep the handle
  in the model, and call `abort()` when a new request makes the old one obsolete.
  Cancellation is cooperative. It does not remove events that are already in the queue,
  and it does not stop work that the shell has already started.
- **Expect stale responses.** An abort cannot stop a response that the shell already
  sent. Tag a request with a version, and ignore a response whose version is not
  current. The weather example does this with a `VersionedInput` helper.
- **Debounce with a timer that you can clear.**
  `crux_time::clock::Time::notify_after(..)` returns a builder and a handle. Keep the
  handle in the model, and call `handle.clear()` before you start a new timer. Handle both `TimerOutcome::Completed` and
  `TimerOutcome::Cleared`.

## 5. Structure the core: nested state machines or features over one model

A core that is larger than one module needs a structure. There are two patterns. Choose
one for each level of the core, and do not mix them at the same level. Two patterns for
one idea make the code harder to read than either pattern alone.

### Choose the pattern

| Question | Nested state machines (5.1) | Features over one model (5.2) |
| --- | --- | --- |
| What are the parts? | Stages that exclude each other: screens, a lifecycle (onboard, active, failed) | Concerns that are all active at the same time: connection, power, radio, light |
| Does one part start work in another part? | No. A part ends with a transition, and the parent starts the next part. | Yes, often. A sleep in `power` turns off the light, closes the connection and turns off the radio. |
| Which state does a part read or change? | Only its own | Its own state and the state of other features |
| Command type of a part | `Command<Effect, ChildEvent>`, lifted with `.map_event(..)` | The root `Command<Effect, Event>` |
| How a child event reaches the root | `.map_event(ParentEvent::Child)` in the parent | `impl From<feature::Event> for crate::Event`, and `.into()` in the feature |
| What the compiler checks | A part touches only its own state and sends only its own events | Less. Each feature has a small `pub fn` boundary, and its state is an enum that has no invalid states |

**Use nested state machines when the parts exclude each other.** The weather example
does this. Each screen owns its state, and its signature shows exactly which events it
can send. You can test a screen with no other state. The cost is the `Started`,
`Outcome` and `Status` types, and a parent that starts each next stage.

**Use features over one model when the parts must act together.** An embedded device or
a background service usually has this shape. If such a core uses nested state machines,
each step that crosses a feature must go up to the parent as an outcome, and the parent
must start the next step. The parent `update` then becomes a large orchestrator. Also,
the type of a feature command cannot hold the commands of the other features, so the
code fights the type system. A feature function that takes `&mut Model` and returns the
root `Command<Effect, Event>` can start the next step itself.

**Both patterns can be in one core at different levels.** For example, a top-level
lifecycle (setup, running) can be a nested state machine, and the running stage can hold
features over one model. Put `.map_event(..)` at the level of the lifecycle only.

**Make invalid states impossible to represent, whichever pattern you use.** Model each
stage and each feature state as an enum, and give each variant only the data for that
state (see Model in section 2). The compiler then rejects code that makes an invalid
state. This protection is the same in both patterns and in a mix of them. In nested
state machines, it adds to the separation of state between parts. In features over one
model, it is the main protection, because any feature can change the state of another.

**`map_effect` solves a different problem.** Use `.map_effect(..)` when a child module has
its own, smaller `Effect` type, for example a reusable module that knows only HTTP. If the
core has one `Effect` enum, it needs no `map_effect`.

### 5.1 Nested state machines

For an app with more than one screen, use the pattern from the weather example. These
types are app code, not `crux_core` API. Copy them from
`examples/weather/shared/src/model/outcome.rs`.

- `Started<State, Event>`: the result of `start()`. It holds the initial state and the
  command that starts its work.
- `Outcome<State, Transition, Event>`: the result of `update()`. It holds a `Status`
  and a command.
- `Status::Continue(State)` keeps the machine running. `Status::Complete(Transition)`
  exits it with a value for the parent.

Rules:

- **Give each state machine a `start()`** that returns `Started`. A `Default` impl gives
  only the state, and the first effects do not start.
- **Make `update` take `self` by value** and return an `Outcome`. The type system then
  makes you put a state back.
- **In the parent, use `std::mem::take`, delegate, then reassign.** Take the current
  model, call the child, and match on the status. For `Continue`, put the child back.
  For `Complete`, call the next state's `start()` and combine the commands with `.and`.
- **Ignore an event that arrives in the wrong state.** Put the model back and return
  `Command::done()`. Do not panic. A late response can always arrive.
- **Lift each child transition into the parent's own transition type.** Each level then
  decides to handle the transition or to pass it up.

### 5.2 Features over one model

This section is recommended practice. No Crux example shows it yet.

Use this pattern for concerns that are active at the same time and start work in each
other. `Model` holds the state of all features, and each feature guards its own part.

- **Give each feature a module** (`features/<name>`). A feature that has state keeps it
  in its own `State` type in `Model`. A feature that gets effect results has its own
  internal `Event` enum and one
  `pub fn update(event, model: &mut Model) -> Command<Effect, Event>`. A type alias
  (`type Cmd = Command<Effect, Event>;`) keeps these signatures short.
- **Wrap each feature's internal events in one root variant**
  (`Event::Power(power::Event)`), and implement `From<power::Event> for crate::Event`.
  Inside the feature, send an event with `.then_send(|x| Event::Done(x).into())`.
- **Make the feature functions take `&mut Model` and return the root
  `Command<Effect, Event>`.** A feature can then call the `pub fn` of another feature
  and combine the commands with `.and(..)`. For example, a sleep in `power` calls
  `light`, `connection` and `wifi` directly.
- **Keep each feature's boundary small.** Other features call only its `pub fn`, and do
  not change its `State` fields directly. Give the state private fields and methods where
  you can.
- **Make each feature's state an enum with no invalid states.** In this pattern, the
  compiler cannot stop one feature from reading another feature's state. A state enum
  makes sure that the state is always valid, whichever feature changes it.
- **Keep `App::update` a dispatcher.** It matches the root event. It sends a public event
  from the shell to the feature function that handles it, and an internal event to the
  feature `update`. It contains no logic of its own.
- **Do not let calls between features make a cycle.** Rust modules permit cycles, so
  the compiler does not stop one. Watch the hub functions that many features call, and
  do not add more work to them.
- **Test whole flows through a test shell.** The features act together, so most
  behaviour shows only in a flow: an event, the effects, the answers, the next event.
  Make the test shell answer each effect as a real shell does, with fakes. Test one
  feature alone only for logic that does not depend on other features.
- **Check rules that span features in the test shell.** The types do not show these
  rules. For example, the test shell can panic if a timer is still active when the core
  asks the shell to sleep.

## 6. Testing

Testing is the main benefit of Crux. Write tests for every event and every effect
result. The tests need no mocks, and a full suite runs in milliseconds.

### Set-up

```toml
[dev-dependencies]
crux_core = { workspace = true, features = ["testing"] }
```

The `testing` feature makes `#[effect]` generate a test trait for
`Command<Effect, Event>`. The macro names the trait after the enum, so `Effect` gives
`EffectTestExt`. The trait exists only with the `testing` feature. Import it from the
module that defines the enum, for example `use crate::effects::EffectTestExt;`.
Run the tests with `cargo nextest run`.

### Test in two levels

Test at the lowest level that shows the behaviour, then test the level above.

1. **Unit-test commands first, where it is useful.** If a function builds a command or
   a builder with its own logic (a typed API client, a chain with `.then_request`, a
   `.map` that turns a result into a domain error, an async `Command::new`), test that
   function alone. Bind the builder to a test event with `.then_send(..)`. Then assert
   on the operations, resolve them, and assert on the events. You need no model or
   `update` for this test.

   ```rust
   #[test]
   fn fetch_maps_unauthorized() {
       let mut cmd: Command<Effect, LocalWeatherEvent> =
           weather_api::fetch(phoenix_location(), api_key())
               .then_send(|result| LocalWeatherEvent::WeatherFetched(Box::new(result)));

       let event = cmd
           .resolve_http(|_op| {
               HttpResult::Ok(HttpResponse::status(401).body(b"Unauthorized").build())
           })
           .expect_one_event();

       let LocalWeatherEvent::WeatherFetched(result) = event else {
           panic!("expected WeatherFetched, got {event:?}");
       };
       assert!(matches!(*result, Err(WeatherError::Unauthorized)));
   }
   ```

   `phoenix_location()` and `api_key()` are local helpers in the test module.

   Skip this level for a command that only issues one operation and sends its output.
   The `update` tests already cover it.

2. **Then test at the `update` level.** Call `update` on a state machine or on the app,
   and assert on the returned command and the new state. These tests check the
   behaviour of the app. They do not test the command internals again.

### Rules

- **Call `update` directly** on the app or on a state machine. Do not use `AppTester`.
  It is kept only for backwards compatibility.
- **Assert on the operation data,** not only on the effect variant. Compare it with
  `assert_eq!` against the value that the code must send, for example
  `weather::build_request(location, &api_key())`.
- **Resolve effects to test the whole flow.** Resolve the effect, take the event that
  the command emits, and pass that event to `update`. Repeat until the flow ends.
- **Write a helper that drives a state machine to a known state,** then start each test
  from there.
- **Test the view model** with `app.view(&model)` and plain equality. Use `insta`
  snapshots for large view models or responses.
- **Start a test in the middle of a flow** by sending an internal event directly, or by
  building the state that you need.
- **Test each state machine on its own,** and also test the top-level `App` for the
  lifecycle transitions.

### Helpers that `EffectTestExt` generates

The `<variant>` part is the `Effect` variant name in snake case. For example,
`GetLocation` gives `expect_get_location`.

| Method | What it does |
| --- | --- |
| `expect_<variant>()` | Asserts that the next effect is this variant. Returns `&mut Self` for chains. |
| `expect_<variant>_with(\|op\| ..)` | Same, and gives the operation to a closure. |
| `expect_only_<variant>()` | Asserts that the next effect is this variant and that no effects or events remain. |
| `expect_only_<variant>_with(\|op\| ..)` | Same, and gives the operation to a closure. |
| `resolve_<variant>(\|op\| output)` | Takes the next effect, which must be this variant, and resolves it with the output. |
| `then_event(\|ev\| ..)` | Gives the next event to a closure. |

`Command` also has these methods with the `testing` feature:

| Method | What it does |
| --- | --- |
| `expect_effect()` / `expect_one_effect()` | Returns the next effect, or the only effect. |
| `expect_event()` / `expect_one_event()` | Returns the next event, or the only event. |
| `expect_no_effects()` / `expect_no_events()` | Asserts that none are ready now. |
| `expect_no_effect_or_events()` | Asserts that nothing is ready now. The command can still be waiting. |
| `expect_done()` | Asserts that the command has finished. |

These `Command` methods are always available. They do not need the `testing` feature:

| Method | What it does |
| --- | --- |
| `effects()` / `events()` | Iterators, for custom assertions. |
| `is_done()` | Returns `true` when the command has finished. |

### Example

```rust
#[test]
fn weather_fetched_stores_data() {
    let (local, mut cmd) = drive_to_fetching_weather();

    let event = cmd
        .resolve_http(|_op| {
            HttpResult::Ok(
                HttpResponse::ok()
                    .body(phoenix_weather_json().as_bytes())
                    .build(),
            )
        })
        .expect_event();
    let (local, _cmd) = local
        .update(event, &api_key())
        .expect_continue()
        .into_parts();

    let LocalWeather::Fetched(loc, _) = local else {
        panic!("Expected Fetched state, got {local:?}");
    };
    assert_eq!(loc, phoenix_location());
}
```

`expect_continue()` and `into_parts()` are test helpers on the app's own `Outcome` type.

### Test HTTP rejections at the correct layer

There are two layers, and each takes a different value.

- **Protocol layer.** `resolve_http` takes the `HttpResult` that the shell sends. A
  rejection is `HttpResult::Ok(HttpResponse::status(401)..build())`. The capability then
  turns it into an `Err`, as a real shell response does.
- **App layer.** If you build an event by hand, use what the app receives. Use
  `crux_http::testing::rejection(status, body)` for a rejection, or
  `rejection_from(response)` when the headers matter. Do not write
  `Ok(ResponseBuilder::with_status(409)..)`. `ResponseBuilder` panics for a 4xx or 5xx,
  because no app can receive one.

### Deterministic simulation

For more confidence, write a small in-memory implementation of each effect (for
example a `HashMap` for the key-value store). Send random events from a seeded random
number generator, inject errors, and check the app invariants. The seed reproduces each
failure.

## 7. Effects handled outside the shell: the effect router and middleware

Most apps send every effect to the shell. Before you handle an effect anywhere else, try
a native implementation in each shell, or a custom capability on top of a lower-level
one such as HTTP.

If an effect must be handled in Rust (for example, by a Rust SDK that is not sans-I/O),
or its data must not be serialized (for example, an opaque handle or a large buffer),
handle it in Rust outside the core. Crux has two APIs for this:

- The `EffectRouter` is the newer API. Use it for new code on native targets.
- Effect middleware (`.handle_effects_using(..)` with an `EffectMiddleware`) is not
  deprecated. Use it on WebAssembly targets. The `counter-routing` example uses the router
  on native targets and the middleware `Bridge` on WebAssembly.

The rules below are for the effect router.

- **Route by effect type in the FFI module,** not in the app. The routing closure
  sends each `Effect` variant to a lane. Follow-up effects go through the same closure.
  The `crux_core::effects::routes` module has the lanes:

  | Lane | Use it for |
  | --- | --- |
  | `Serialized` | The default. The shell gets serialized effects. Use it as the fall-through arm. |
  | `Parked` | Data that you must not serialize. A custom FFI resolves it by `ParkedEffectId`. |
  | `Buffer` | Requests that the caller drains and handles synchronously, for example in tests. |
  | A Rust handler | An effect that the core handles itself. It resolves through `ResolveSink`. |

- **Group the lanes in a type that implements `Routes`,** and build the router with
  `EffectRouter::new`. The routes hold a `Weak` reference to the router, which is in an
  `Arc`.
- **Resolve once for a request, once for each item of a stream, and never for a
  notification.**
- **Resolve asynchronously, for example from a background thread.** A synchronous
  resolve inside the routing call can recurse, because each resolve can produce new
  effects that go through the same closure. The `counter-routing` example sends each
  request to a worker thread.
- **Narrow the FFI `Effect` enum** if Rust handles a variant on a target. Remove the
  variant from the FFI enum and `panic!` on it in the `From` impl. The shell then never
  receives that effect at runtime. Type generation reads the app `Effect` enum, so it
  still emits the variant. The `counter-routing` example narrows the enum only on
  targets that are not WebAssembly.
- **Test the app without the router.** The routed effect is a normal effect in command
  and `update` tests.

The reference is `examples/counter-routing/shared/src/ffi.rs` and the `crux_core::effects`
module docs. For middleware, refer to the `crux_core::middleware` module docs and the
middleware page of the book.

## 8. Layout

Follow the layout of the weather example:

```text
shared/src/
  app.rs              # App impl that delegates to Model, plus top-level tests
  model/mod.rs        # Event, Model, lifecycle update
  model/outcome.rs    # Started, Outcome, Status
  model/<stage>/...   # one module for each state machine, with its tests
                      # (or features/<name>/ for features over one model, see section 5)
  effects/mod.rs      # Effect enum
  effects/<capability>/mod.rs      # operations
  effects/<capability>/command.rs  # builders
  effects/http/<api>/              # typed API clients
  view/               # ViewModel and From<&Model> impls
  ffi.rs              # bridge (or effect router, see section 7)
  bin/codegen.rs      # type generation
```

Keep each test module next to the code that it tests.

## Check before you finish

1. `update` and `view` do no I/O, and no command reads or writes the model.
2. Each operation has one output and a declared kind, and each `Effect` variant wraps
   one operation.
3. If a shell uses a bridge or type generation, internal events have `#[serde(skip)]`
   and `#[facet(skip)]`, enums that cross the boundary have `#[repr(C)]`, and the shared
   crate forwards the `facet_typegen` feature.
4. Each command with its own logic has a unit test. Each new event and each effect
   result has a test that calls `update` and asserts on the operation data and the new
   state.
5. HTTP error handling reads rejections on the `Err` side, and the tests build them at
   the correct layer.
6. `cargo build`, `cargo clippy --all-targets` and `cargo nextest run` pass.
