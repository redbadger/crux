# Heap probe

Runs the app the way the firmware shell does, with a counting allocator, and reports peak
heap. It connects, opens the SSE stream, receives updates, then makes bursts of presses with
their POSTs in flight. It encodes each request for the link, as the firmware does. It builds
crux_core and crux_http with the firmware's `no_std` features.

```sh
just heap-probe       # on the host (8-byte pointers, so an upper bound)
just heap-probe-wasm  # as wasm32-wasip1 under Node: 4-byte pointers, as on the nRF52840
```

(Run both from the directory above.) The results are in `../SPIKE_NOTES.md`.
