# Counter Observability

An interactive terminal shell for the `counter-routing` example which records Crux runtime
observability through `tracing` and writes a Perfetto trace file.

This is an incomplete prototype. Event spans track command lifetimes, and render
notifications are instantaneous observations. Requests and streams currently have
start observations only; their span endings are not yet tracked. The checked-in
trace and session notes below are historical artifacts, not validation of the
current tracing semantics.

Run it from this directory:

```sh
cargo run -- counter-observability.pftrace
```

Then try a short session:

```text
get
inc
random 3
watch
sse 5
done
quit
```

Open the generated `counter-observability.pftrace` at <https://ui.perfetto.dev>.

The shell handles HTTP, random, render, and server-sent event effects locally. It adds a
small delay before resolving asynchronous effects so the effect spans are visible in Perfetto.

## Recorded Test Session

This is the session used to produce `counter-observability.pftrace` while
developing the example:

```sh
cargo run --quiet -- counter-observability.pftrace
```

```text
get
inc
dec
random 3
watch
sse 5
sse 8
done
quit
```

The resulting trace should include:

- shell events for `Get`, `Increment`, `Decrement`, `Random`, and `StartWatch`
- update spans caused by both shell events and command-emitted events
- render, HTTP, random, and SSE effect spans
- a longer-lived SSE effect span with multiple `EffectResolved` events
- nested command spans for the `Random -> UpdateBy -> HTTP fan-out -> Set -> Update` chain

For a denser trace, the checked-in `counter-observability.pftrace` was generated
with:

```text
get
inc
inc
dec
random 4
random -3
watch
sse 10
inc
sse 12
dec
sse 9
random 2
done
quit
```
