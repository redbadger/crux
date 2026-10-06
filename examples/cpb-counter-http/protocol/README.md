# Protocol

The messages between the firmware and the gateway, shared by both (`no_std`).

- The GATT service and characteristic UUIDs, and the advertised name.
- `ToGateway` (`Http`, `Sse`, `Cancel`) and `ToDevice` (`Http`, `SseChunk`, `SseDone`). Each
  carries a request id, and the HTTP payloads are crux_http's own protocol types.
- Framing: postcard bytes with a `u16` length prefix, cut into chunks. The `Reassembler`
  treats chunks as a byte stream, so each side can use any chunk size.
- `SseRequest`/`SseResponse`: the Server-Sent Events operation, used by the app and the gateway.

`cargo test -p cpb-protocol` round-trips every message at every chunk size.
