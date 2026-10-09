# Heap probe

Runs the counter-http core (`../../shared`, without std, as the firmware builds it) the way
the firmware shell does, with a counting allocator, and reports peak heap. The gateway
connects (`Get`, `StartWatch`), the SSE stream receives updates, then come bursts of presses
with their POSTs in flight, and finally the link drops and the stream ends. It encodes each
request for the link, as the firmware does.

```sh
just heap-probe       # on the host (8-byte pointers, so an upper bound)
just heap-probe-wasm  # as wasm32-wasip1 under Node: 4-byte pointers, as on the nRF52840
```

(Run both from the directory above.) The results are in `../SPIKE_NOTES.md`.
