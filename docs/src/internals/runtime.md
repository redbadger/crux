# Capability Runtime

In the previous sections we focused on building applications in Crux and using
it's public APIs to do so. In this and the following chapters, we'll look at how
the internals of Crux work, starting with the capability runtime.

The capability runtime is a set of components whose job it is to facilitate the
processing of effects to present the two perspectives we previously mentioned:

* For the core, the shell appears to be a platform with a message based system interface
* For the shell, the core appears as a stateful library responding to events with request for side-effects

There are a few chalenges to solve in order to facilitate this interface. First,
each run of the `update` function can call several capabilities, and the effects
produced are expected to be emitted together and processed concurrently, so the calls can't be blocking. Second, each effect requested from a capability may
require multiple round-trips between the core and shell to conclude and we
don't want to require a call to `update` per round trip, so we need some
ability to "suspend" execution in capabilities while waiting for an effect to
be fulfilled. The third challenge is dispatch - effects started in a particular
capability, once resolved, need to continue execution in the same capability.

Given this concurrency and execution suspension, an async interface seems like
a good candidate. Capabilities request work from the shell, `.await`
the results, and continue their work when the result has arrived. The call to
`request_from_shell` or `stream_from_shell` translates into an effect request
returned from the current core "transaction" (one call to `process_event`
or `resolve`).

```admonish note
In this chapter, we will focus on the runtime and the core interface and ignore
the serialization, bridge and FFI, and return to them in the following sections.
The examples will assume a Rust based shell.
```

## Async runtime

One of the fairly unique aspects of Rust's async is the fact that it doesn't
come with a bundled runtime. This is in recognition that asynchronous execution
is useful in various different scenarios, and no one runtime can serve all of
them. Crux takes advantage of this and brings it's own runtime, tailored to the
execution of side-effects on top of a message based interface.

For a deeper background on Rust's async architecture, we recommend... _TODO
recommed a good source (one that describes building an executor.)_ We will
assume you are familiar with the basic ideas and mechanics of async here.

The job of an async runtime is to manage a number of tasks, each driving one
future to completion. This management is done by an executor, which is
responsible for scheduling the futures and `poll`ing them _at the right time_ to
drive their execution forward. Most "grown up" runtimes will do this on a number
of threads in a thread pool, but for Crux, we run in the context of a single
function call (of the app's `update` function) and potentially in a webassembly
context which is single threaded anyway, so our runtime only needs to poll all
the tasks sequentially, to see if any of them need to continue.

Polling all the tasks would work, and in our case wouldn't even be that
inefficient, but the async system is set up to avoid unnecessary polling of
futures with one additional concept - wakers. A waker is a mechanism which can
be used to signal the executor that something a given task is waiting on has
changed, and the task's future should be polled, because it will be able to
proceed. This is how "at the right time" is decided.

In our case there's a single situation which causes such a change - a result has
arrived from the shell, for a particular effect requested earlier.

## One effect's life cycle

So, step by step, our strategy for the capabilities to handle effects is:

1. A capability `spawn`s a task and submits a future with some code to run
1. The new task is scheduled to be polled next time the executor runs
1. The executor goes through the list of ready tasks until it gets to our task and polls it
1. The future runs to the point wher the first async call is `await`ed. In
capabilities, this can only be a future returned from one of the calls to
request something from the shell, or a future resulting from a composition of
such futures (with combinators like `select` or `join`).
1. The shell request future's first step is to create the request and prepare
it to be sent. We will look at the mechanics of the sending in a minute, but
for now it's only important that part of this request is a callback used to
resolve it.
1. The request future, as part of the first poll by the executor, sends the
request does so. As there is no result from the shell yet, it returns a pending state and the task is suspended.
1. The request is passed on to the shell to resolve (as a return from `process_event` or `resolve`)
1. Eventually, the shell has a result ready for the request and asks the core to
`resolve` the request.
1. The request callback mentiond above is executed, puts the provided result
onto the future's state, and calls the future's waker, also stored in the future's state, to wake the future up.
1. The executor runs again (asked to do so by the core `resolve` API after
calling the callback), and polls the awakened future.
1. the future sees there is now a result available and returns a ready result,
continuing the execution of the original task.

The cycle may repeat a few times, but eventually the original task completes and
is removed.

This is probably a lot to take in, but the basic gist is that capability futures
(the ones submitted to `spawn`) always pause on request futures (the ones
returned from `request_from_shell` et al.), which submit requests. Resolving
requests updates the state of the original future and wakes it up to continue
execution.

With that in mind we can look at the individual moving parts and how they
communicate.

## Spawning tasks on the executor

The first step for anythning to happen is spawning a task from a capability.
Each capability is created with a `CapabilityContext`. This is the definition:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/capability/mod.rs:399:413}}
```

There are a couple sending ends of channels for requests and events, which we
will get to soon, and also a spawner, from the executor module. The `Spawner`
looks like this:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/capability/executor.rs:17:20}}
```

also holding a sending end of a channel, this one for `Task`s. The final piece
of the puzzle is the executor itself:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/capability/executor.rs:13:15}}
```

This is the receiving end of the channel from the spawner.

The executor has a single method, `run_all`:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/capability/executor.rs:58:79}}
```

besides the locking and waker mechanics, the gist is quite simple - while there
are tasks in the ready_queue, poll the future held in each one.

The last interesting bit of this part is how the waker is provided to the `poll`
call. The `waker_ref` creates a waker which, when asked to wake up, will call
this method on the task:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/capability/executor.rs:48:56}}
```

this simply enqueues the task again for processing on the next run.

The only missing piece is when does the `run_all` get called, and the answer is
in the `Core` API implementation. Both `process_event` and `resolve` call
`run_all` after their respective task - calling the app's `update` function, or
resolving the given task.

Now we know how the futures get executed, suspended and resumed, we can examine
the flow of information between capabilities and the Core API calls layered on
top.

## Requests flow from capabilities to the core

TODO
