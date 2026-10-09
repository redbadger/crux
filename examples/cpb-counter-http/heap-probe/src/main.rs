//! Host-side heap probe for the cpb-counter-http core. Drives `app::Counter` the way the
//! firmware shell does (including encoding each request for the link, as `link.rs`
//! does) and reports peak heap. Run it as wasm32-wasip1 for 4-byte pointers, as on the
//! nRF52840; on a 64-bit host the numbers are an upper bound.

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use ble_protocol::{SseResponse, ToGateway, encode};
use cpb_counter_http_app::{Counter, Effect, Event};
use crux_core::{Core, Request};
use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};

struct Tracking;
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Tracking {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let now = CURRENT.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
        PEAK.fetch_max(now, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Tracking = Tracking;

#[derive(Default)]
struct Shell {
    http: BTreeMap<u16, Request<HttpRequest>>,
    sse: Option<Request<ble_protocol::SseRequest>>,
    next_id: u16,
    renders: usize,
}

impl Shell {
    fn handle(&mut self, core: &Core<Counter>, effects: Vec<Effect>) {
        for effect in effects {
            self.next_id += 1;
            let id = self.next_id;
            match effect {
                Effect::Render(_) => {
                    let _ = core.view();
                    self.renders += 1;
                }
                Effect::Http(request) => {
                    let frame = encode(&ToGateway::Http {
                        id,
                        request: request.operation.clone(),
                    });
                    drop(frame);
                    self.http.insert(id, request);
                }
                Effect::ServerSentEvents(request) => {
                    drop(encode(&ToGateway::Sse {
                        id,
                        request: request.operation.clone(),
                    }));
                    self.sse = Some(request);
                }
                // Only after a stream ends, which this probe does last.
                Effect::Delay(_) => {}
            }
        }
    }

    fn answer_all(&mut self, core: &Core<Counter>, value: &mut i64) {
        for (_, mut request) in std::mem::take(&mut self.http) {
            *value += 1;
            let body = format!(r#"{{"value":{value},"updated_at":1672531200000}}"#);
            let result = HttpResult::Ok(
                HttpResponse::ok()
                    .header("content-type", "application/json")
                    .body(body)
                    .build(),
            );
            let effects = core.resolve(&mut request, result).expect("resolves");
            self.handle(core, effects);
        }
    }

    fn sse_chunk(&mut self, core: &Core<Counter>, value: i64) {
        let chunk = format!("data: {{\"value\":{value},\"updated_at\":1672531200000}}\n\n");
        let mut request = self.sse.take().expect("stream open");
        let effects = core
            .resolve(&mut request, SseResponse::Chunk(chunk.into_bytes()))
            .expect("resolves");
        self.sse = Some(request);
        self.handle(core, effects);
    }
}

fn live(base: usize) -> usize {
    CURRENT.load(Ordering::Relaxed) - base
}

fn peak(base: usize) -> usize {
    PEAK.load(Ordering::Relaxed) - base
}

fn main() {
    let base = CURRENT.load(Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);

    let core: Core<Counter> = Core::new();
    let mut shell = Shell::default();
    let mut value = 0;

    let effects = core.process_event(Event::Switch(true));
    shell.handle(&core, effects);
    let effects = core.process_event(Event::Connected);
    shell.handle(&core, effects);
    println!(
        "connected, GET + SSE in flight: peak {} B, live {} B",
        peak(base),
        live(base)
    );

    shell.answer_all(&core, &mut value);
    println!(
        "GET answered, SSE open:         peak {} B, live {} B",
        peak(base),
        live(base)
    );

    for _ in 0..20 {
        value += 1;
        shell.sse_chunk(&core, value);
    }
    println!(
        "20 SSE updates:                 peak {} B, live {} B",
        peak(base),
        live(base)
    );

    for burst in [1usize, 5, 20, 20, 1] {
        for i in 0..burst {
            let event = if i % 2 == 0 {
                Event::Increment
            } else {
                Event::Decrement
            };
            let effects = core.process_event(event);
            shell.handle(&core, effects);
        }
        let in_flight = shell.http.len();
        let peak_in_flight = peak(base);
        shell.answer_all(&core, &mut value);
        println!(
            "burst of {burst:>2} presses ({in_flight:>2} POSTs in flight, SSE open): peak so far {peak_in_flight} B, live after settle {} B",
            live(base)
        );
    }

    // The link drops: the shell ends the stream with Done.
    if let Some(mut request) = shell.sse.take() {
        let effects = core
            .resolve(&mut request, SseResponse::Done)
            .expect("resolves");
        shell.handle(&core, effects);
    }
    let effects = core.process_event(Event::Disconnected);
    shell.handle(&core, effects);
    println!("disconnected, stream ended:     live {} B", live(base));
    println!("renders: {}", shell.renders);
}
