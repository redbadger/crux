# Counter Observability

An interactive terminal shell for `counter-routing` that records Crux observations
through `tracing` and exports Chrome trace JSON for Perfetto.

To reproduce the sample trace, run from this directory:

```sh
just trace
```

This plays the checked-in `session.txt` through the local shell and writes
`output/session.json`. Open it using **Open trace file** at
<https://ui.perfetto.dev>. The scenario is repeatable; recorded timings vary
between runs. Generated output is ignored by version control.

To explore interactively instead:

```sh
cargo run -- counter-observability.json
```

Try this session, then open the JSON file at <https://ui.perfetto.dev>:

```text
get
inc
random 3
watch
sse 5
random 2
sse 8
done
quit
```

The shell handles HTTP, random, render, and server-sent events locally, with a
small delay before responses. No server is needed.

## What the trace means

- Each event starts a span covering `App::update` and its returned command,
  labelled with the event variant name from `EventName`. It ends when that
  command's owned work finishes or is cleaned up after cancellation.
- Causal arrows connect commands through emitted app events. A command does not
  wait for, or cancel, commands returned by later updates. The source may have
  ended before the destination starts.
- Request and stream effects have spans covering the command's wait, ending on
  completion, stream termination, or dropping the wait. These spans do not claim
  that external shell work has stopped.
- Stream responses appear as instant markers when consumed by the command.
  Responses buffered in a channel have no marker until consumed.
- Notifications, including renders, appear as instant markers on command tracks.

Look for `Random` ending while the caused `UpdateBy` command continues. `StartWatch`
and its `ServerSentEvents` effect remain open until `done`, alongside independent
updates caused by stream responses and other shell events.

## Exporter scope

The example's tracing layer buffers a session and writes complete slices on
separate tracks, so overlapping asynchronous work is represented correctly.
Effect slices carry `parent_context_id`; command slices carry `caused_by`.
Causal arrows connect spans, with their source anchored at the originating
span's start: they do **not** claim to timestamp the precise emission of the event.
The exporter retains span data for the session, not live tracing handles.

This is a small example exporter, not an OpenTelemetry SDK integration. Perfetto
supports this format directly: <https://perfetto.dev/docs/getting-started/other-formats>.
The old checked-in `.pftrace` file is a historical artifact; generate fresh JSON
when reviewing these semantics.
