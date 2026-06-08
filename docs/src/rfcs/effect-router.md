# RFC: Effect Router and shell-owned effect handling

This RFC proposes a new approach for mixed effect handling, based on routing effects
by type to explicit handlers.

The main goal, similar to middleware, is to support advanced cases where some effects
should be handled inside the Core, or where request/result data is not serializable
across the FFI boundary, while keeping the default, serialized FFI path simple.

We are primarily looking for feedback on the direction and model, rather than
the exact public API details (we can iron that out on on a later pull request).

## Why?

The advanced need we keep running into is two-fold:

1. not all effects should always be handled on the shell side (e.g. need to use an existing Rust SDK)
2. some request/result data cannot or should not be serialized across the bridge (e.g. need to fetch a GPU rendering surface, font information, large byte buffer, or other opaque / non-serializable data)

Middleware was our first attempt at solving this. It works for the core-side effect handling, but
it has quickly shown design limitations:

- routing follow-up effects after `resolve` is difficult to reason about
- stacking layers is mechanically complex
- mixing bridge interfaces (e.g. serialized and custom FFI), while probably _possible_, is just too difficult
- the stack metaphor does not align with how effect handling is usually chosen

The main design direction of the new approach is that handling is typically aligned with the
**effect type**. We think that in practice, people want handling to differ between effect categories
(http, time, files, database, ...), but rarely between requests of the same
category ("some HTTP requests go one way, other HTTP requests another").

So instead of a middleware pipeline, this RFC proposes a **router with
handlers**, where routing happens by effect type. The main difference is that
in the middleware, the metaphor was a stack or pipeline where some effects were
teed off to the side. In this approach, there is an explicit fan out by type.

If this approach lands, the plan is to deprecate the middleware API and replace
it with effect routing.

## Design decisions and constraints

1. Keep `App` implementations unaware of handling mechanics. This allows them to
   change independently, and differ between platforms for the same app.
1. Keep the main FFI type as the assembly point, like we did with middleware.
   The type is user-owned, explicit and the users get full control over their FFI surface this way.
1. Keep Bridge as the default/simple path. No change
1. Make Router an opt-in upgrade for advanced cases. We don't want early users to need to immediately
   understand the full extent of the mechanics of effect handling. The FFI side of the shared crate
   should be considered boilerplate, and not need to be touched until the user realises the need to
   do something special.
1. Support mixed handling of effects in one app:
   1. Serialized FFI using Facet type generation, like today
   1. New typed/opaque "lane" with custom FFI with no built-in constraints
   1. Core-local Rust lane, like the one enabled by middleware
   1. Other lanes, if they come up
1. Keep migration cost minimal for existing bridge-based apps, and make the upgrade path from
   simple to advanced use smooth

### Non-goals

In this RFC we're not looking to:

- Finalise all low-level public API visibility details
- Design any level of macro sugar
- Finalise all failure/cancellation policy details
- Optimise effect registry internals in this phase.

## Proposed architecture

As with the Middleware approach, the Shell is responsible for creating a
Core instance and providing an implementation of a Core-defined trait defining
the callback interface to the Shell. The trait is now extended with multiple callbacks

```rust,no_run,noplayground
pub(crate) trait CameraShell: Send + Sync {
    fn process_serialized_effects(&self, bytes: Vec<u8>);
    fn process_camera_effect(&self, effect: CameraEffect);
}
```

We introduce a `Router` wrapper around `Core`. The router is
constructed with a routing closure which decides how each effect type is
handled.

```rust,no_run,noplayground
let router = Router::new(core, {
    let shell = shell.clone();
    let serialized_registry = serialized_registry.clone();
    let camera_registry = camera_registry.clone();

    |weak_router| {
        let fs_store = file_store::FileStoreHandler::new(weak_router.clone());

        move |effect| match effect {
            // Core-side effect, processed in Rust
            app::Effect::FileStore(request) => {
                fs_store.process_file_store(request);
            }
            // Shell-side effect, but with a custom FFI for opaque data
            app::Effect::Camera(request) => {
                let (id, op) = camera_registry.register(request);
                shell.process_camera_effect(CameraEffect { id, operation: op });
            }
            // Original serialized FFI
            effect => {
                let serialized_effect = SerializedEffect::try_from(effect)
                    .expect("non-serialized effects are handled above");

                let request = serialized_registry.register(serialized_effect);
                let mut bytes = vec![];
                Format::serialize(&mut bytes, &vec![request])
                    .expect("serialized effect request should encode");

                shell.process_serialized_effects(bytes);
            }
        }
    }
});
```

The closure can:

- send effects to shell callbacks from the aforementioned shell trait
- send effects to core-local async handlers
- fall through to a serialized lane (also implemented by shell trait)

Async handlers resolve back through the router, so follow-up effects are
automatically routed using the same policy.

### Effect lanes

In principle, there are a few "lanes" of effect handling, which have common
mechanics. In the proof of concept code, there are three:

#### Serialized lane

The serialized lane keeps the bridge-like behavior:

- request is registered with an id
- shell receives bytes
- shell resolves with id and bytes
- registry remembers how to deserialize for that id and resumes the right
  suspended request

This is the default lane and remains the primary onboarding path. It will also
typically act as the default match arm in the routing closure.

#### Opaque typed lane

The typed lane supports payloads/results that are awkward or undesirable to
serialize (for example pointer-style handles or opaque references), using typed
callbacks and typed resolve methods. These are fully in the user's control and
Crux has no opinions on them.

A generic Registry type is provided to support recording the effect requests under
an ID and store the `Resolve` continuation (which cannot cross an FFI boundary of
any kind).

#### Core-local lane

The local lane allows effect requests to be handled by Rust code outside
`App::update`, including async/background work. Handler completion resolves back
through the router.

```rust,no_run,noplayground
pub(crate) struct FileStoreHandler {
    jobs_tx: Sender<Request<app::StoreFile>>,
}

impl FileStoreHandler {
    pub(crate) fn new<S>(sink: Weak<S>) -> Self
    where
        S: ResolveSink<app::StoreFile> + Send + Sync + 'static,
    {
        let (jobs_tx, jobs_rx) = unbounded();

        thread::spawn(move || worker(jobs_rx, sink));

        Self { jobs_tx }
    }

    pub(crate) fn process_file_store(&self, request: Request<app::StoreFile>) {
        self.jobs_tx
            .send(request)
            .expect("file store worker queue disconnected")
    }
}
```

### Runtime flow

The key trick to this is that all follow up effects after any type of resolution
are passed through the routing layer and dispatched to the right handler again.

At runtime, the flow is:

1. `update(event)` produces effects
2. router dispatches each effect to the selected lane/handler
3. shell/local handler eventually resolves the request
4. resolution goes through router, so the effect runtime is moved forward
5. router collets and routes follow-up effects
6. repeat until settled (no more follow up effects)

This flow was (almost) possible without any of this scaffolding, but the router
makes it less fragile, by enforcing the routing of follow-up effects and advancing
the runtime at the right times.

## Open questions

1. Should we keep double-dispatch in the serialized case (router dispatch, then
   shell match on serialized effect), or move toward one FFI callback pair per
   effect type?
2. What is the best way to expose runtime advance for router-managed
   `resolve(id, bytes)` flows, without creating an awkward public "half-method"
   or requiring users to remember extra steps?
3. What registry implementation should back effect id lookup in the production
   API (`HashMap` in prototype, potentially a more specialised structure later)?
4. Should we support synchronous handling? Currently a loop in synchronously handled
   effect would lead to an infinite mutual recursion, which will need addressing by
   either supporting it, or explicitly preventing it like the effect middleware does
5. Failure/cancellation handling in the routing setup. This is not perfect in the basic
   Bridge case either

Considerations for the first question: The big pro would be that different effects could have separate
IDs, and we could prevent resolving one effect with a value of another by mistake. The
downside is that each new effect would need to get registered in a few places – the effect type
the match arm of the router and a new pair of FFI methods in the trait and in the FFI impl.

## Next steps

1. Gather feedback on this approach and the open questions.
2. Prove the architecture in a full example, including FFI generation workflow,
   to validate the integration story.
3. Refine public API shape based on that validation, then proceed with
   middleware deprecation planning.
