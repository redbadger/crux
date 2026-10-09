# BLE protocol

The messages between a Crux app's firmware and the [BLE gateway](../ble_gateway/), shared by
both. It's `no_std` and builds for thumbv7em (the firmware), wasm (the gateway) and the host
(tests). It knows nothing about any particular app; the
[`cpb-counter-http`](../../examples/cpb-counter-http/) spike is its first user.

- The GATT service and characteristic UUIDs, and the advertised name.
- `ToGateway` (`Http`, `Sse`, `Cancel`) and `ToDevice` (`Http`, `SseChunk`, `SseDone`). Each
  carries a request id, and the HTTP payloads are crux_http's own protocol types.
- Framing: postcard bytes with a `u16` length prefix, cut into chunks. The `Reassembler`
  treats chunks as a byte stream, so each side can use any chunk size.
- `SseRequest`/`SseResponse`: the wire copy of the Server-Sent Events operation. An app keeps
  its own SSE types; its firmware maps the request's url across, and the responses back.

It's a member of the root workspace, so it builds with the root's lockfile and lints. Its
dependencies are declared explicitly, with `default-features = false`, rather than taken from
the root's `[workspace.dependencies]`, whose entries keep their std defaults.

```sh
just test    # round-trips every message at every chunk size
just check   # fmt + clippy (pedantic) on the host, thumbv7em and wasm32
```
