# Command Runtime

In the previous sections we focused on building applications in Crux and using
its public APIs to do so. In this and the following chapters, we'll look at how
the internals of Crux work, starting with the command runtime.

The command runtime is a set of components that process effects, presenting
the two perspectives we previously mentioned:

- For the core, the shell appears to be a platform with a message based system
  interface
- For the shell, the core appears as a stateful library responding to events
  with requests for side-effects

There are a few challenges to solve in order to facilitate this interface.
First, each run of the `update` function returns a `Command` which may
contain several concurrent tasks, each requesting effects from the shell.
The requested effects are expected to be emitted together, and each batch
of effects will be processed concurrently, so the calls can't be blocking.
Second, each effect may require multiple round-trips between the core and
shell to conclude and we don't want to require a call to `update` per
round trip, so we need some ability to "suspend" execution while waiting
for an effect to be fulfilled. The ability to suspend effects introduces a
new challenge — effects which are suspended need, once resolved, to
continue execution in the same async task.

Given this concurrency and execution suspension, an async interface seems
like a good candidate. Commands request work from the shell, `.await` the
results, and continue their work when the result has arrived. The call to
`request_from_shell` or `stream_from_shell` translates into an effect
request returned from the current core "transaction" (one call to
`process_event` or `resolve`).

```admonish note
In this chapter, we will focus on the runtime and the core interface and ignore
the serialisation, bridge and FFI, and return to them in the following sections.
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
drive their execution forward. General-purpose runtimes like Tokio do
this on a number of threads in a thread pool, but in Crux, we run in
the context of a single function call (of the app's `update` function)
and potentially in a WebAssembly context which is single-threaded
anyway, so our runtime only needs to poll all the tasks sequentially,
to see if any of them need to continue.

Polling all the tasks would work, and in our case wouldn't even be that
inefficient, but the async system is set up to avoid unnecessary polling of
futures with one additional concept - wakers. A waker is a mechanism which can
be used to signal to the executor that something that a given task is waiting on
has changed, and the task's future should be polled, because it will be able to
proceed. This is how "at the right time" from the above paragraph is decided.

In our case there's a single situation which causes such a change - a result has
arrived from the shell, for a particular effect requested earlier.

```admonish warning
Always use the Command APIs provided by Crux for async work (see the
[capabilities](../part-2/capabilities.md) chapter). Using other async
APIs can lead to unexpected behaviour, because the resulting futures
are not tied to Crux effects. Such futures will resolve, but only
after the next shell request causes the Crux executor to execute.
```

```admonish info
If you want to depend on a crate that requires a standard runtime like
Tokio, you can integrate it through an effect via
[middleware](../part-3/middleware.md).
```

## One effect's life cycle

So, step by step, our strategy for commands to handle effects is:

1. A `Command` creates a task containing a future with some code to
   run (via `Command::new` or `ctx.spawn`)
1. The new task is scheduled to be polled next time the executor runs
1. The executor goes through the list of ready tasks until it gets to
   our task and polls it
1. The future runs to the point where the first async call is
   `await`ed. In commands, this _should_ only be a future returned
   from one of the calls to request something from the shell, or a
   future resulting from a composition of such futures (through async
   method calls or combinators like `select` or `join`).
1. The shell request future's first step is to create the request and
   prepare it to be sent. We will look at the mechanics of the sending
   shortly, but for now it's only important that part of this request
   is a callback used to resolve it.
1. The request future, as part of the first poll by the executor,
   sends the request to be handed to the shell. As there is no result
   from the shell yet, it returns a pending state and the task is
   suspended.
1. The request is passed on to the shell to resolve (as a return value
   from `process_event` or `resolve`)
1. Eventually, the shell has a result ready for the request and asks
   the core to `resolve` the request.
1. The request's resolve callback is executed, sending the provided
   result through an internal channel. The channel wakes the future's
   waker, which enqueues the task for processing on the executor.
1. The executor runs again (asked to do so by the core's `resolve` API
   after calling the callback), and polls the awoken future.
1. The future sees there is now a result available and continues the
   execution of the original task until a further await or until
   completion.

The cycle may repeat a few times, depending on the command
implementation, but eventually the original task completes and is
removed.

This is probably a lot to take in, but the basic gist is that command
futures (the ones created by `Command::new` or `ctx.spawn`) always
pause on request futures (the ones returned from `request_from_shell`
et al.), which submit requests. Resolving requests updates the state
of the original future and wakes it up to continue execution.

With that in mind we can look at the individual moving parts and how
they communicate.

## Spawning tasks on the executor

The first step for anything to happen is creating a `Command` with a
task. Each task runs within a `CommandContext`, which provides the
interface for communicating with the shell and the app:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/command/context.rs:command_context}}
```

There are sending ends of channels for effects and events, and also
a sender for spawning new tasks. The `rc` field is a reference
counter used to track whether any contexts are still alive
(indicating the command may still produce more work).

A `Command` is itself an async executor, managing a set of tasks:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/command/mod.rs:command}}
```

It holds the receiving ends of the effect and event channels, along
with the executor internals: a `Slab` of tasks, a ready queue of
task IDs, and a spawn queue for new tasks.

Each `Task` is a simple data structure holding a future and some
coordination state:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/command/executor.rs:task}}
```

Tasks are spawned by `CommandContext::spawn`:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/command/context.rs:spawn}}
```

After constructing a task with the future returned by the closure,
it is sent to the command's spawn queue. A `JoinHandle` is returned,
which can be used to await the task's completion or abort it.

The command runs all tasks to completion (or suspension) with
`run_until_settled`:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/command/executor.rs:run_until_settled}}
```

The method first checks if the command has been aborted. If not, it
loops: spawning any new tasks from the spawn queue, then polling
each ready task. Tasks that complete are removed. Tasks that are
suspended wait to be woken.

The waking mechanism is provided by `CommandWaker`:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/command/executor.rs:command_waker}}
```

When a task's future needs to be woken (because a shell response has
arrived), the waker sends the task's ID back to the ready queue and
also wakes the parent waker (used when the command is running as a
stream inside another command).

While there are a lot of moving pieces involved, the basic mechanics
are relatively straightforward — tasks are submitted either by
`Command::new`, `ctx.spawn`, or awoken by arriving responses to the
requests they submitted. The queue of tasks is processed whenever
`run_until_settled` is called. This happens in the `Core` API
implementation: both `process_event` and `resolve` trigger it as
part of their processing.

Now we know how the futures get executed, suspended and resumed, we
can examine the flow of information between commands and the Core
API calls layered on top.

## Requests flow from commands to the shell

The key to understanding how the effects get processed and executed
is to name all the various pieces of information, and discuss how
they are wrapped in each other.

The basic inner piece of the effect request is an _operation_. This
is the intent which the command is submitting to the shell. Each
operation has an associated _output_ value, with which the operation
request can be resolved. There are multiple capabilities in each
app, and in order for the shell to easily tell which capability's
effect it needs to handle, we wrap the operation in an _effect_. The
`Effect` type is a generated enum based on the app's set of
capabilities, with one variant per capability. It allows us to
multiplex (or type erase) the different typed operations into a
single type, which can be `match`ed on to process the operations.

Finally, the effect is wrapped in a _request_ which carries the
effect, and an associated _resolve_ callback to which the output
will eventually be given. We discussed this callback in the previous
section — its job is to send the result through an internal channel,
waking up the paused future. The request is the value passed to the
shell, and used as both the description of the effect intent, and
the "token" used to resolve it.

Each task in a command has access to a `CommandContext`, which holds
the sending ends of channels for effects and events. When a task
calls `request_from_shell`, the context creates a `Request`
containing the operation and a resolve callback, wraps it in the
app's `Effect` type (via the `From` trait), and sends it through the
effects channel. The `Command` collects these effects and surfaces
them to the `Core`.

Looking at the core itself:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/core/mod.rs:core}}
```

The `Core` holds a `root_command` — a single long-lived `Command`
onto which all commands returned from `update` are spawned. This
root command acts as the top-level executor, collecting all effects
and events across all active commands.

## A single update cycle

To piece all these things together, let's look at processing a
single call from the shell. Both `process_event` and `resolve` share
a common step advancing the command runtime.

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

First, we drain events from the root command (which internally runs
all ready tasks before collecting). There can be new events because
we just returned a command from `update` (which may have immediately
sent events) or resolved some effects (which woke up suspended
futures that then sent events).

For each event, we call `update` again, spawning the returned
command onto the root command, and drain any further events produced.
This continues until no more events remain.

Finally, we collect all of the effect requests submitted in the
process and return them to the shell.

## Resolving requests

We've now seen everything other than the mechanics of resolving
requests. The resolve callback is carried by the request as a
`RequestHandle`, tagged by the expected number of resolutions:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/core/resolve.rs:resolve}}
```

A `RequestHandle` can be `Never` (for notifications that don't
expect a response), `Once` (for one-shot requests), or `Many` (for
streaming requests). Resolving a `Once` handle consumes it, turning
it into `Never` to prevent double-resolution.

Here's how the resolve callback is set up in `request_from_shell`:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/command/context.rs:request_from_shell}}
```

The callback sends the output through an `mpsc` channel. On the
receiving end, the `ShellRequest` future is waiting — when the value
arrives, the channel wakes the future's waker, which schedules the
task on the executor to continue.

In the next chapter, we will look at how this process changes when
Crux is used via an FFI interface where requests and responses need
to be serialised in order to pass across the language boundary.
