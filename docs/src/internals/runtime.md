# Capability Runtime

In the previous sections we focused on building applications in Crux and using
its public APIs to do so. In this and the following chapters, we'll look at how
the internals of Crux work, starting with the capability runtime.

The capability runtime is a set of components that processes effects, presenting
the two perspectives we previously mentioned:

- For the core, the shell appears to be a platform with a message based system
  interface
- For the shell, the core appears as a stateful library responding to events
  with request for side-effects

There are a few challenges to solve in order to facilitate this interface.
First, each run of the `update` function can call several capabilities. The
requested effects are expected to be emitted together, and each batch of effects
will be processed concurrently, so the calls can't be blocking. Second, each effect requested from
a capability may require multiple round-trips between the core and shell to
conclude and we don't want to require a call to `update` per round trip, so we
need some ability to "suspend" execution in capabilities while waiting for an
effect to be fulfilled. The ability to suspend effects introduces a new
challenge - effects started in a particular capability and suspended, once
resolved, need to continue execution in the same capability.

Given this concurrency and execution suspension, an async interface seems like a
good candidate. Capabilities request work from the shell, `.await` the results,
and continue their work when the result has arrived. The call to
`request_from_shell` or `stream_from_shell` translates into an effect request
returned from the current core "transaction" (one call to `process_event` or
`resolve`).

```admonish note
In this chapter, we will focus on the runtime and the core interface and ignore
the serialization, bridge and FFI, and return to them in the following sections.
The examples will assume a Rust based shell.
```

## Async runtime

One of the fairly unique aspects of Rust's async is the fact that it doesn't
come with a bundled runtime. This is recognising that asynchronous execution is
useful in various different scenarios, and no one runtime can serve all of them.
Crux takes advantage of this and brings its own runtime, tailored to the
execution of side-effects on top of a message based interface.

For a deeper background on Rust's async architecture, we recommend the
[Asynchronous Programming in Rust](https://rust-lang.github.io/async-book/)
book, especially the chapter about
[executing futures and tasks](https://rust-lang.github.io/async-book/02_execution/01_chapter.html).
We will assume you are familiar with the basic ideas and mechanics of async
here.

The job of an async runtime is to manage a number of tasks, each driving one
future to completion. This management is done by an executor, which is
responsible for scheduling the futures and `poll`ing them _at the right time_ to
drive their execution forward. Most "grown up" runtimes will do this on a number
of threads in a thread pool, but in Crux, we run in the context of a single
function call (of the app's `update` function) and potentially in a webassembly
context which is single threaded anyway, so our baby runtime only needs to poll
all the tasks sequentially, to see if any of them need to continue.

Polling all the tasks would work, and in our case wouldn't even be that
inefficient, but the async system is set up to avoid unnecessary polling of
futures with one additional concept - wakers. A waker is a mechanism which can
be used to signal to the executor that something that a given task is waiting on
has changed, and the task's future should be polled, because it will be able to
proceed. This is how "at the right time" from the above paragraph is decided.

In our case there's a single situation which causes such a change - a result has
arrived from the shell, for a particular effect requested earlier.

```admonish warning
Always use the capability APIs provided by Crux for async work (see the
[capabilities](../guide/capability_apis.md) chapter). Using other async APIs can
lead to unexpected behaviour, because the resulting futures are not tied to crux
effects. Such futures will resolve, but only after the next shell request causes
the crux executor to execute.
```

## One effect's life cycle

So, step by step, our strategy for the capabilities to handle effects is:

1. A capability `spawn`s a task and submits a future with some code to run
1. The new task is scheduled to be polled next time the executor runs
1. The executor goes through the list of ready tasks until it gets to our task
   and polls it
1. The future runs to the point where the first async call is `await`ed. In
   capabilities, this _should_ only be a future returned from one of the calls
   to request something from the shell, or a future resulting from a composition
   of such futures (through async method calls or combinators like `select` or
   `join`).
1. The shell request future's first step is to create the request and prepare it
   to be sent. We will look at the mechanics of the sending shortly, but for now
   it's only important that part of this request is a callback used to resolve
   it.
1. The request future, as part of the first poll by the executor, sends the
   request to be handed to the shell. As there is no result from the shell yet,
   it returns a pending state and the task is suspended.
1. The request is passed on to the shell to resolve (as a return value from
   `process_event` or `resolve`)
1. Eventually, the shell has a result ready for the request and asks the core to
   `resolve` the request.
1. The request callback mentioned above is executed, puts the provided result in
   the future's mutable state, and calls the future's waker, also stored in the
   future's state, to wake the future up. The waker enqueues the future for
   processing on the executor.
1. The executor runs again (asked to do so by the core's `resolve` API after
   calling the callback), and polls the awoken future.
1. the future sees there is now a result available and continues the execution
   of the original task until a further await or until completion.

The cycle may repeat a few times, depending on the capability implementation,
but eventually the original task completes and is removed.

This is probably a lot to take in, but the basic gist is that capability futures
(the ones submitted to `spawn`) always pause on request futures (the ones
returned from `request_from_shell` et al.), which submit requests. Resolving
requests updates the state of the original future and wakes it up to continue
execution.

With that in mind we can look at the individual moving parts and how they
communicate.

## Spawning tasks on the executor

The first step for anything to happen is spawning a task from a capability. Each
capability is created with a `CapabilityContext`. This is the definition:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/capability/mod.rs:capability_context}}
```

There are a couple of sending ends of channels for requests and events, which we
will get to soon, and also a spawner, from the executor module. The `Spawner`
looks like this:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/capability/executor.rs:spawner}}
```

also holding a sending end of a channel, this one for `Task`s.

Tasks are a fairly simple data structure, holding a future and another sending
end of the tasks channel, because tasks need to be able to submit themselves
when awoken.

```rust,no_run,noplayground
{{#include ../../../crux_core/src/capability/executor.rs:task}}
```

Tasks are spawned by the Spawner as follows:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/capability/executor.rs:spawning}}
```

after constructing a task, it is submitted using the task sender.

The final piece of the puzzle is the executor itself:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/capability/executor.rs:executor}}
```

This is the receiving end of the channel from the spawner.

The executor has a single public method, `run_all`:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/capability/executor.rs:run_all}}
```

besides the locking and waker mechanics, the gist is quite simple - while there
are tasks in the ready_queue, poll the future held in each one.

The last interesting bit of this part is how the waker is provided to the `poll`
call. The `waker_ref` creates a waker which, when asked to wake up, will call
this method on the task:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/capability/executor.rs:wake}}
```

this is where the task resubmits itself for processing on the next run.

While there are a lot of moving pieces involved, the basic mechanics are
relatively straightforward - tasks are submitted either by the spawner, or the
futures awoken by arriving responses to the requests they submitted. The queue
of tasks is processed whenever `run_all` is called on the executor. This happens
in the `Core` API implementation. Both `process_event` and `resolve` call
`run_all` after their respective task - calling the app's `update` function, or
resolving the given task.

Now we know how the futures get executed, suspended and resumed, we can examine
the flow of information between capabilities and the Core API calls layered on
top.

## Requests flow from capabilities to the shell

The key to understanding how the effects get processed and executed is to name
all the various pieces of information, and discuss how they are wrapped in each
other.

The basic inner piece of the effect request is an _operation_. This is the
intent which the capability is submitting to the shell. Each operation has an
associated _output_ value, with which the operation request can be resolved.
There are multiple capabilities in each app, and in order for the shell to
easily tell which capability's effect it needs to handle, we wrap the operation
in an _effect_. The `Effect` type is a generated enum based on the app's set of
capabilities, with one variant per capability. It allows us to multiplex (or
type erase) the different typed operations into a single type, which can be
`match`ed on to process the operations.

Finally, the effect is wrapped in a _request_ which carries the effect, and an
associated _resolve_ callback to which the output will eventually be given. We
discussed this callback in the previous section - its job is to update the
paused future's state and resume it. The request is the value passed to the
shell, and used as both the description of the effect intent, and the "token"
used to resolve it.

Now we can look at how all this wrapping is facilitated. Recall from the
previous section that each capability has access to a `CapabilityContext`, which
holds a sending end of two channels, one for events - the `app_channel` and one
for requests - the `shell_channel`, whose type is `Sender<Request<Op>>`. These
channels serve both as thread synchronisation and queueing mechanism between the
capabilities and the core of crux. As you can see, the requests expected are
typed for the capability's operation type.

Looking at the core itself, we see their `Receiver` ends.

```rust,no_run,noplayground
{{#include ../../../crux_core/src/core/mod.rs:core}}
```

One detail to note is that the receiving end of the requests channel is a
`Receiver<Ef>`. The channel has an additional feature - it can map between the
input types and output types, and, in this case, serve as a multiplexer,
wrapping the operation in the corresponding Effect variant. Each sending end is
specialised for the respective capability, but the receiving end gets an already
wrapped `Effect`.

## A single update cycle

To piece all these things together, lets look at processing a single call from
the shell. Both `process_event` and `resolve` share a common step advancing the
capability runtime.

Here is `process_event`:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/core/mod.rs:process_event}}
```

and here is `resolve`:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/core/mod.rs:resolve}}
```

The interesting things happen in the common `process` method:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/core/mod.rs:process}}
```

First, we run all ready tasks in the executor. There can be new tasks ready
because we just ran the app's update function (which may have spawned some task
via capability calls) or resolved some effects (which woke up their suspended
futures).

Next, we drain the events channel (where events are submitted from capabilities
by `context.update_app`) and one by one, send them to the `update` function,
running the executor after each one.

Finally, we collect all of the effect requests submitted in the process and
return them to the shell.

## Resolving requests

We've now seen everything other than the mechanics of resolving requests. This
is ultimately just a callback carried by the request, but for additional type
safety, it is tagged by the expected number of resolutions

```rust,no_run,noplayground
{{#include ../../../crux_core/src/core/resolve.rs:resolve}}
```

We've already mentioned the resolve function itself briefly, but for
completeness, here's an example from `request_from_shell`:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/capability/shell_request.rs:resolve}}
```

Bar the locking and sharing mechanics, all it does is update the state of the
future (`shared_state`) and then calls `wake` on the future's waker to schedule
it on the executor.

In the next chapter, we will look at how this process changes when Crux is used
via an FFI interface where requests and responses need to be serialised in order
to pass across the language boundary.
