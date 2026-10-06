# Spike: counter_http on the Circuit Playground Bluefruit, over BLE

Throwaway spike, following `examples/cpb-counter`. Evidence for an RFC, not mergeable code.

The goal is `examples/counter-http` (a shared counter on crux-counter.fly.dev: GET, POST
/inc and /dec with an optimistic update, SSE) running as no_std firmware, using
**crux_http's own API**. The board has no IP stack, so its HTTP and SSE effects go over
BLE to a Chrome page. That page is itself a Crux app, which acts as a gateway and performs
them with fetch.

## Phase 1: a no_std subset of crux_http

### Result

`cargo check -p crux_http --no-default-features --target thumbv7em-none-eabihf` passes.
With default features nothing changes: `cargo test -p crux_http` gives the same counts as
before (97 unit, 15 integration, 76 doc), `cargo test --workspace` passes (483), the
`shared` tests of counter-http, counter-middleware, counter-routing and weather pass, and
counter-http's typegen produces byte-identical TypeScript.

The no_std path has its own test, `crux_http/tests/nostd_command.rs`
(`cargo test -p crux_http --no-default-features --test nostd_command`, 7 tests). It
compiles to nothing under std, so it can only run against the no_std code.

### Blockers, measured

| Dependency | no_std? | What happened |
|---|---|---|
| `http` 1.5 | **no** | `compile_error!("std feature currently required, support for no_std may be added later")` in its `lib.rs` |
| `mime` 0.3 | **no** | No features at all, uses std |
| `serde_qs` 1.1 | no | Only used by `.query()` |
| `async-trait` | n/a | Only used by `middleware` and the std `EffectSender` |
| `encoding_rs` | yes | But only used by `body_string`'s charset decoding, which is std-only anyway |
| `url` 2.5.8 | **yes** | `default-features = false`, `Url::parse`/`join` work |
| `serde_json` | yes | `features = ["alloc"]` |
| `serde_bytes` | yes | `features = ["alloc"]` |
| `derive_builder` 0.20 | **yes, on stable** | `default-features = false, features = ["alloc"]` plus `#[builder(no_std)]`. Its docs say nightly-only, but that is out of date |
| `thiserror` 2 | yes | |
| `facet-generate-attrs` 0.18 | **no (our own crate)** | It depends on `facet` with default features, which turns `facet/std` on for the whole build: 2,787 errors in facet-core on thumbv7em. **Upstream ask:** `default-features = false` there |

`http` and `mime` are not peripheral. They are in crux_http's public API: `Request` is built
on `HeaderMap`/`Method`, `Response` keeps `StatusCode`/`HeaderMap`/`Version`, `HttpError::Http`
carries a `Box<HeaderMap>`, and the crate does `pub use http` and `pub use mime`.

### What was done (the "subset" option)

The options were patching no_std forks of `http`/`mime` in (same API, a fork to maintain),
replacing them with crux_http-owned types (breaking for every user), or a no_std subset.
This spike does the subset:

- New `std` feature in `default`, carrying `http`, `mime`, `serde_qs`, `async-trait`,
  `facet-generate-attrs` and the std features of the rest. `encoding`, `facet_typegen` and
  `http-types` imply it. crux_core is depended on with `default-features = false`, and its `std`
  comes back through ours.
- `facet` and `thiserror` spell out versions instead of `.workspace = true`, the same
  workspace-dependency trap as cpb-counter's SPIKE_NOTES §5.
- **Shared between std and no_std, the same code:** `protocol` (`HttpRequest`,
  `HttpResponse`, `HttpResult`, `HttpHeader` and their builders), `expect`, and `HttpError`.
  - `HttpRequestBuilder::body_json` now writes `content-type: application/json` as literals
    rather than via `http::header::CONTENT_TYPE` and `mime::APPLICATION_JSON`, which are the same bytes.
  - `#[facet(typegen::bytes)]` became `#[cfg_attr(feature = "std", facet(typegen::bytes))]`.
  - `HttpError` stays one type. Only the skipped `Http` variant's `headers` field differs:
    `Box<HeaderMap>` under std, `Vec<HttpHeader>` without. So does the accessor:
    `header(&str) -> Option<&str>` and `headers() -> Option<&[HttpHeader]>` without std.
    The **serialized** variants (`Url`, `Io`, `Timeout`) are identical, so a no_std device
    and a std gateway agree on the wire.
  - `HttpResult` therefore serializes the same in both.
- **std only:** `client`, `middleware`, `config`, the legacy `Request`/`RequestBuilder`,
  `Body`, `RawResponse`, `testing`, `shell`, `compat`, `.query()`, `.content_type(Mime)`,
  `body_form`, and the `http`/`mime`/`Method` re-exports.
- **no_std twins, at the same paths** (`src/nostd/`, swapped in with `#[path]`):
  - `crux_http::Response<T>`: `status: u16`, `headers: Vec<HttpHeader>`, `body: Option<T>`,
    with `status()`, `header(&str)`, `headers()`, `content_type() -> Option<&str>`, `body()`,
    `take_body()`, `with_body()`, `body_bytes()`, `body_string()` (UTF-8 only), `body_json()`,
    `PartialEq`/`Eq`, and `TryFrom<HttpResponse>`. The same rules apply as in std: 4xx/5xx
    becomes `HttpError::Http` (with a `message` of just the number, because there is no reason-phrase
    table), a status outside 100..=999 becomes `InvalidStatusCode`, and a non-UTF-8 body
    becomes `HttpError::Io("could not decode body as utf-8")`, the variant and message std produces.
  - `crux_http::command::{Http, RequestBuilder}`: `Http::get/head/post/put/delete/patch/
    options/trace/connect(impl AsRef<str>)`, and `Http::request(method: impl AsRef<str>, url)`
    instead of `(Method, Url)`. The builder has `header`, `body_bytes`, `body_string`,
    `body_json`, `expect_string`, `expect_json` and `build`. It builds the protocol `HttpRequest`
    directly and sends what the std builder would send: header names lowercased (as
    `http::HeaderName` does), `header` replacing an existing one, the same `content-type`
    per `body_*`, and the URL normalised through `Url` (so `https://host` goes out as
    `https://host/`).

So **std and no_std share names and call shapes, not types**. counter_http's HTTP code
(`Http::get(url).expect_json().build().then_send(Event::Set)`, `Http::post(url)…`,
`Url::parse(..).join(..)`, `crux_http::Result<crux_http::Response<Count>>`,
`response.take_body()`, matching on `HttpError`) compiles against either. Anything that
touches a `StatusCode`, `HeaderValue` or `Mime` does not: `response.status().as_u16()` is
std-only, and `response.status()` is already a `u16` without std.

### For the RFC

1. **The subset is a second API to keep in step.** Every builder method added to the std
   `RequestBuilder` needs a decision for the no_std one. A proper version would probably
   make crux_http's public types its own (status as `u16` or a small newtype, headers as an
   ordered `Vec`), with `http` interop behind a feature, so that one API serves both. That
   is a breaking change and needs its own RFC.
2. **`http` may go no_std upstream** (the `compile_error!` says "may be added later"); if it
   does, most of the twin code can go.
3. **facet-generate-attrs** should depend on `facet` with `default-features = false`. It is
   ours, and it is a one-line change.
4. `derive_builder`'s no_std mode works on stable, despite its docs.
5. The no_std builder has no middleware. Whether `crux_http::middleware` (async-trait,
   `Box<dyn>`, `Arc`) is wanted on MCUs is the same question as crux_core's effect
   middleware (cpb-counter SPIKE_NOTES §10.6).

## Phase 2: the BLE link

### 2a. Go/no-go: passed (2026-10-06)

`firmware/src/bin/ble_probe.rs`: TrouBLE 0.8 host + Nordic's SoftDevice Controller
(`nrf-sdc 0.4`, `nrf-mpsl 0.4`, crates.io releases, which now line up with embassy-nrf
0.11 / embassy-sync 0.8, so the git pin TrouBLE's own examples use was not needed).
Flashed by UF2 at 0x26000 on the same board (bootloader 0.9.0, S140 6.1.1): it
advertises, Chrome connects through Web Bluetooth, and writes to `rx` come back as `tx`
notifications (`firmware/tools/ble-probe.html`). A 17-byte echo takes 36–50 ms round trip
(write without response → notify, as measured in the page), about one to two connection intervals.

So **MPSL's timing-critical RADIO/TIMER0/RTC0 interrupts survive the MBR → disabled
S140 forwarding**, as GPIOTE did in cpb-counter. The resident SoftDevice can stay; the
board needs no bootloader or SoftDevice update.

- Manifest differences from cpb-counter: `cortex-m` loses `critical-section-single-core`
  (`nrf-mpsl/critical-section-impl` provides it), and embassy-nrf gains `unstable-pac`, `rt`.
  HFCLK is no longer forced to the crystal in `embassy_nrf::init`, because MPSL owns CLOCK;
  the WS2812 timing is fine on the internal oscillator.
- LFCLK is from the RC oscillator with MPSL's recommended calibration intervals: the CPB
  has no 32 kHz crystal (CircuitPython `mpconfigboard.h`: `BOARD_HAS_32KHZ_XTAL (0)`).
- MPSL takes RTC0, TIMER0, TEMP, RADIO, CLOCK, EGU0, the RNG and PPI channels 17–31; the
  app keeps RTC1 (the embassy time driver), PWM0 (the NeoPixels) and GPIOTE.
- Size: text 97.7 KB, data 4.2 KB, bss 14.3 KB, almost all of it the controller
  library (the peripheral-only variant).
- GATT service: 128-bit UUID `744aa621-…-a43b54bb1544`, `tx` (notify, device → gateway)
  and `rx` (write / write without response, gateway → device), each up to 244 bytes. The
  device is the peripheral because Web Bluetooth can only be a central, so the SIG HTTP
  Proxy Service (0x1823, where the proxy is the GATT server) does not fit.

### 2b/2c. Wire protocol, gateway, firmware link: HTTP round trip works (2026-10-06)

- `protocol/` (no_std): `ToGateway { Http, Sse, Cancel }` and `ToDevice { Http, SseChunk,
  SseDone }`. Each carries an `id`, and the HTTP payloads are crux_http's own
  `HttpRequest`/`HttpResult`. Encoding is postcard plus a `u16` length prefix. The receiver
  treats chunks as a byte stream, so each direction picks its own chunk size: the device
  notifies at ATT MTU − 3, and the gateway writes 20 bytes at a time because Chrome does not
  expose the MTU. A POST /inc is 85 bytes and its answer 77. There are 6 host tests,
  including round trips at every chunk size.
- `gateway/core`: a Crux app. Effects: `BleConnect` (stream), `BleWrite`, crux_http's
  `Http`, and `ServerSentEvents`. It re-emits each decoded request with
  `Command::request_from_shell` / `stream_from_shell`, so it is a generic proxy. Writes are
  serialised through an outbox, because Chrome runs one GATT operation at a time. SSE streams
  are aborted (`AbortHandle`) on `Cancel` or disconnect. An answer that arrives after a
  disconnect is dropped. There are 7 host tests.
  - Finding: after `AbortHandle::abort()`, the *first* `resolve` of that stream's request
    still returns `Ok` (the aborted task is only dropped when the command next runs). The
    next one returns `Err(FinishedMany)`. So a shell that stops reading when `resolve` fails
    closes the fetch one chunk late, which is fine but worth knowing.
  - `#[operation(output = Result<(), String>)]` does not parse (the macro splits on the
    comma), so the gateway uses a type alias. That is arguably a feature: facet_generate
    cannot emit more than one monomorphisation of a generic, so a named output type is what
    typegen'd apps need anyway. Note that an alias is erased before facet sees it, so for
    typegen the output has to be a real named type (e.g. `enum WriteResult { Ok, Err(String) }`),
    not an alias. The gateway is Rust-only, so the alias is enough here.
- `gateway/web`: Leptos with web-sys Web Bluetooth (`--cfg=web_sys_unstable_apis`). It
  remembers the device, so a reconnect needs no chooser. Its HTTP and SSE handlers are
  counter-http's, extended with request bodies and all methods.
- `firmware/src/link.rs`: radio set-up, the GATT server, framing, and two channels to the
  app. **TrouBLE's `notify` returns `Ok` and sends nothing when the central has not
  subscribed**, so the link is "up" only when the CCCD write arrives (it does surface as a
  GATT write event).
- `firmware` `link-check` binary: on connect it sends a raw `ToGateway::Http` GET (no Crux
  yet). On hardware, the gateway logged `#1 → GET https://crux-counter.fly.dev/`, then
  `#1 ← 200 (40 B)`, and the device decoded the 200. Repeats worked. SSE works too: with the
  stream open, every change made from another client reached the board as an `SseChunk`. Size: text 107 KB, bss 47.6 KB (32 KB heap).

## Phase 3: counter_http on the board

### Result (2026-10-06)

`app/` is counter_http's Crux app in a `#![no_std]` crate. **Its HTTP code is counter_http's
unchanged** (`Http::get(API_URL).expect_json().build().then_send(Event::Set)`,
`Http::post(url)…`, `Url::parse(..).join(..)`, `crux_http::Result<crux_http::Response<Count>>`,
`response.take_body()`), compiled against crux_http's no_std subset from phase 1, and it builds
for thumbv7em. On the board, through the gateway:

- Connecting sends a GET and opens the SSE stream. The count shows as up to 10 pixels, green
  for positive and red for negative.
- Buttons A and B update optimistically: dim pixels with D13 lit while pending, then solid once
  the POST's 200 returns. One run logged 22 POSTs (#3–#24), each answered with 200. One
  press is one request (cpb-counter's press-and-release debounce).
- Changes made from other clients arrive over SSE (48-byte chunks; the server also sends
  3-byte keep-alives).
- With the network down, the POST failed (`IO error: TypeError: Failed to fetch`) without
  freezing the board, and later presses worked again.

Deliberate differences from counter_http (all in `app/src/lib.rs`'s header):
`updated_at` is epoch millis rather than chrono; the view is pixels; `Set(Err)` sets an error
flag (the pixels alternate red and blue) instead of `panic!`, because `panic-halt` would freeze
the board; there are `Connected`/`Disconnected` events (the link; connecting does what
counter_http's shells do at start-up, `Get` then `StartWatch`) and a `Switch` event for the
brightness. counter_http's four tests are ported (`app/src/tests.rs`), resolving with raw
protocol types because crux_http's `testing` helpers need std. Seven new tests cover SSE split
across chunks, connect/disconnect, the error pattern and the view. The SSE command keeps a line
buffer across chunks (`app/src/sse.rs`), where counter_http uses `async-sse` (std) on each chunk.

### The SSE stream is reopened after it ends (with back-off)

First run: when the network dropped, the gateway's SSE fetch ended (`SSE closed by the
server`), the app's stream command finished, and nothing reopened it (`0 open streams`).
Requests still worked, but the board stopped seeing other clients' changes until the next BLE
reconnect. counter_http behaves the same way (a browser user reloads the page; checked), but
the board has nobody to do that.

Fix, in this app only (counter_http is unchanged): `StartWatch` runs the
`ServerSentEvents` stream inside a `Command::new` task (via `StreamBuilder::into_stream`), so
the core sees the stream end (`WatchEnded`). If the link is still up, it asks the shell for a
`Delay` (cpb-counter's custom operation, resolved with embassy timers), then reopens the
stream. The delay starts at 1 s, doubles to at most 30 s, and resets on the next update from
the server. Each stream has an id, so a stream end or timer left over from before a reconnect
is ignored, and `StartWatch` does nothing while a stream is open. There are 3 tests (back-off
growth and reset, stale timers after a reconnect, no retry while disconnected).

On hardware, with Wi-Fi off and then on: `#2 ← SSE closed by the server`, then retries #3–#6
each closed at once while the network was down, spaced further apart each time. #7 succeeded
when the network came back, and chunks reached the board again without a BLE reconnect.
Also checked on hardware: reconnecting after the gateway tab is closed and reopened, and the
red/blue error pattern for a request that fails while the network is down.

### Sizes

| | text | rodata | data | bss |
|---|---|---|---|---|
| `ble-probe` (radio only) | 97.7 KB (incl. rodata) | | 4.2 KB | 14.3 KB |
| `link-check` (+ protocol, no app) | 107 KB (incl. 1.2 KB rodata) | | 4.2 KB | 47.6 KB (32 KB heap) |
| `cpb-counter-http` | 208 KB | 116 KB | 4.2 KB | 80.4 KB (64 KB heap) |

(With the SSE back-off, `size` reports text 327 KB, bss 80.4 KB.)

`cargo bloat --crates` (.text 203 KiB): std/core 23 KiB, **url 21.7 + idna 19.1 +
icu_normalizer 5.1 KiB**, nrf_sdc_sys 19.9, trouble_host 19.7, the app 17.9, the shell 15.7,
nrf_mpsl_sys 9.3, embassy_futures 9.1, embassy_executor 7.9, serde_json 7.7, **crux_core
3.1 KiB**. Almost all of the extra 115 KB of `.rodata` arrives with the app, in anonymous
constants: Unicode/IDNA tables (url) and serde_json's tables are the likely bulk, not
attributed exactly. So **`url` is the biggest single cost of using crux_http's API on an
MCU**: about 46 KiB of code plus most of the rodata, just to parse and join one base URL. An
RFC could consider letting the no_std builder accept already-valid URL strings without
going through `url` (IDNA in particular).

### Heap

`heap-probe/` drives `app::Counter` as the firmware shell does, including encoding each
request for the link. As wasm32-wasip1 (4-byte pointers, as on the nRF52840):

| | peak | live after |
|---|---|---|
| connected, GET + SSE in flight | 4.0 KB | 3.6 KB |
| GET answered, SSE open, 20 SSE updates | 5.0 KB | 3.3 KB |
| 5 presses in flight | 14.0 KB | 3.3 KB |
| 20 presses in flight | 45.4 KB | 3.8 KB |
| disconnected | | 2.4 KB |

So the cost is about 1.6 KB per HTTP request in flight (crux_http's builder, the Command's
channels and boxed future, the protocol request and its frame), roughly the same as one
cpb-counter `Delay`. The firmware has a 64 KB heap and refuses more than 16 requests in flight
(`MAX_IN_FLIGHT`): the core gets `HttpError::Io` at once and shows the error pattern,
instead of running out of heap, which with `panic-halt` would freeze the board. The probe
builds crux_core and crux_http with the firmware's (no_std) features, but runs on a host
allocator, so allocator overhead is not counted.
