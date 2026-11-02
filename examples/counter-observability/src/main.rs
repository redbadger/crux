use std::{
    collections::HashMap,
    fs::File,
    io::{self, Write as _},
    sync::Mutex,
    thread,
    time::Duration,
};

use anyhow::{Context as _, Result, bail};
use counter_routing::{
    Core, Counter, Effect, Event, RandomNumber,
    http::protocol::{HttpRequest, HttpResponse, HttpResult},
    sse::SseResponse,
};
use crux_core::{CoreRuntimeObserver, Request, RuntimeContextId};
use tracing::Span;
use tracing_subscriber::prelude::*;

const RESPONSE_DELAY: Duration = Duration::from_millis(15);

fn main() -> Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "counter-observability.pftrace".to_string());

    install_perfetto_layer(&output)?;

    let core = Core::<Counter>::new().with_runtime_observer(TracingObserver::default());

    let mut shell = Shell::new(core);
    shell.run()?;

    println!("wrote {output}");
    println!("open it at https://ui.perfetto.dev");

    Ok(())
}

fn install_perfetto_layer(output: &str) -> Result<()> {
    let file = Mutex::new(File::create(output).with_context(|| format!("create {output}"))?);
    let layer = tracing_perfetto::PerfettoLayer::new(file)
        .with_debug_annotations(true)
        .with_filter_by_marker(is_perfetto_marker);

    tracing_subscriber::registry()
        .with(layer)
        .try_init()
        .context("install tracing subscriber")
}

fn is_perfetto_marker(field_name: &str) -> bool {
    field_name == "perfetto"
}

struct Shell {
    core: Core<Counter>,
    server_count: isize,
    timestamp_ms: i64,
    next_random: isize,
    sse_request: Option<Request<counter_routing::sse::SseRequest>>,
}

impl Shell {
    fn new(core: Core<Counter>) -> Self {
        Self {
            core,
            server_count: 0,
            timestamp_ms: 1_672_531_200_000,
            next_random: 3,
            sse_request: None,
        }
    }

    fn run(&mut self) -> Result<()> {
        print_help();
        self.print_view();

        loop {
            print!("counter> ");
            io::stdout().flush().context("flush prompt")?;

            let mut input = String::new();
            if io::stdin().read_line(&mut input).context("read command")? == 0 {
                self.close_sse()?;
                break;
            }

            let mut parts = input.split_whitespace();
            let Some(command) = parts.next() else {
                continue;
            };

            match command {
                "get" => self.dispatch(Event::Get)?,
                "inc" | "+" => self.dispatch(Event::Increment)?,
                "dec" | "-" => self.dispatch(Event::Decrement)?,
                "random" => {
                    if let Some(value) = parts.next() {
                        self.next_random = value.parse().context("parse random value")?;
                    }
                    self.dispatch(Event::Random)?;
                }
                "watch" => self.dispatch(Event::StartWatch)?,
                "sse" => {
                    let value = parts
                        .next()
                        .map(str::parse)
                        .transpose()
                        .context("parse SSE value")?
                        .unwrap_or_else(|| self.server_count + 1);
                    self.send_sse(value)?;
                }
                "done" => self.close_sse()?,
                "view" => self.print_view(),
                "help" | "?" => print_help(),
                "quit" | "exit" | "q" => {
                    self.close_sse()?;
                    break;
                }
                command => {
                    println!("unknown command: {command}");
                    print_help();
                }
            }
        }

        Ok(())
    }

    fn dispatch(&mut self, event: Event) -> Result<()> {
        println!("event: {event:?}");
        let effects = self.core.process_event(event);
        self.handle_effects(effects)
    }

    fn handle_effects(&mut self, effects: Vec<Effect>) -> Result<()> {
        for effect in effects {
            self.handle_effect(effect)?;
        }

        Ok(())
    }

    fn handle_effect(&mut self, effect: Effect) -> Result<()> {
        match effect {
            Effect::Render(_) => {
                self.print_view();
                Ok(())
            }
            Effect::Http(mut request) => {
                println!(
                    "http: {} {}",
                    request.operation.method, request.operation.url
                );
                thread::sleep(RESPONSE_DELAY);

                let response = self.http_response(&request.operation)?;
                let effects = self.core.resolve(&mut request, response)?;
                self.handle_effects(effects)
            }
            Effect::Random(mut request) => {
                println!("random: returning {}", self.next_random);
                thread::sleep(RESPONSE_DELAY);

                let effects = self
                    .core
                    .resolve(&mut request, RandomNumber(self.next_random))?;
                self.handle_effects(effects)
            }
            Effect::ServerSentEvents(request) => {
                println!("sse: subscribed to {}", request.operation.url);
                self.sse_request = Some(request);
                Ok(())
            }
        }
    }

    fn http_response(&mut self, request: &HttpRequest) -> Result<HttpResult> {
        match (request.method.as_str(), request.url.as_str()) {
            ("GET", "https://crux-counter.fly.dev" | "https://crux-counter.fly.dev/") => {}
            ("POST", "https://crux-counter.fly.dev/inc") => self.server_count += 1,
            ("POST", "https://crux-counter.fly.dev/dec") => self.server_count -= 1,
            _ => bail!("unexpected HTTP request: {request:?}"),
        }

        self.timestamp_ms += 1_000;
        let body = format!(
            r#"{{"value":{},"updated_at":{}}}"#,
            self.server_count, self.timestamp_ms
        );

        Ok(HttpResult::Ok(HttpResponse::ok().body(body).build()))
    }

    fn send_sse(&mut self, value: isize) -> Result<()> {
        let Some(mut request) = self.sse_request.take() else {
            println!("sse: not subscribed; run `watch` first");
            return Ok(());
        };

        println!("sse: value {value}");
        thread::sleep(RESPONSE_DELAY);

        self.server_count = value;
        self.timestamp_ms += 1_000;
        let chunk = format!(
            "data: {{\"value\":{},\"updated_at\":{}}}\n\n",
            self.server_count, self.timestamp_ms
        );

        let effects = self
            .core
            .resolve(&mut request, SseResponse::Chunk(chunk.into_bytes()))?;
        self.sse_request = Some(request);
        self.handle_effects(effects)
    }

    fn close_sse(&mut self) -> Result<()> {
        let Some(mut request) = self.sse_request.take() else {
            return Ok(());
        };

        println!("sse: closing subscription");
        thread::sleep(RESPONSE_DELAY);

        let effects = self.core.resolve(&mut request, SseResponse::Done)?;
        self.handle_effects(effects)
    }

    fn print_view(&self) {
        let view = self.core.view();
        println!("view: {} confirmed={}", view.text, view.confirmed);
    }
}

fn print_help() {
    println!("commands:");
    println!("  get          fetch the current counter");
    println!("  inc | +      optimistic increment, then fake HTTP confirmation");
    println!("  dec | -      optimistic decrement, then fake HTTP confirmation");
    println!("  random [n]   resolve Random as n, then run n HTTP updates");
    println!("  watch        start the SSE subscription");
    println!("  sse [n]      send one fake SSE counter update");
    println!("  done         close the SSE subscription");
    println!("  view         print the current view model");
    println!("  quit         close SSE if needed and write the trace");
}

#[derive(Default)]
struct TracingObserver {
    spans: Mutex<HashMap<RuntimeContextId, Span>>,
}

impl CoreRuntimeObserver for TracingObserver {
    fn event_started(
        &self,
        context_id: RuntimeContextId,
        parent_context_id: RuntimeContextId,
        name: &'static str,
    ) {
        let parent = self.span(parent_context_id);
        let tracing_span = if let Some(parent) = parent {
            tracing::info_span!(
                parent: &parent,
                "crux.event",
                perfetto = true,
                context_id = context_id.0,
                parent_context_id = parent_context_id.0,
                crux_name = name,
            )
        } else {
            tracing::info_span!(
                "crux.event",
                perfetto = true,
                context_id = context_id.0,
                parent_context_id = parent_context_id.0,
                crux_name = name,
            )
        };

        self.spans
            .lock()
            .expect("tracing observer lock poisoned")
            .insert(context_id, tracing_span);
    }

    fn event_ended(&self, context_id: RuntimeContextId) {
        self.spans
            .lock()
            .expect("tracing observer lock poisoned")
            .remove(&context_id);
    }

    fn effect_started(
        &self,
        context_id: RuntimeContextId,
        parent_context_id: RuntimeContextId,
        name: &'static str,
    ) {
        let parent = self.span(parent_context_id);
        let tracing_span = if let Some(parent) = parent {
            tracing::info_span!(
                parent: &parent,
                "crux.effect",
                perfetto = true,
                context_id = context_id.0,
                parent_context_id = parent_context_id.0,
                crux_name = name,
            )
        } else {
            tracing::info_span!(
                "crux.effect",
                perfetto = true,
                context_id = context_id.0,
                parent_context_id = parent_context_id.0,
                crux_name = name,
            )
        };

        self.spans
            .lock()
            .expect("tracing observer lock poisoned")
            .insert(context_id, tracing_span);
    }

    fn notification(&self, parent_context_id: RuntimeContextId, name: &'static str) {
        let parent = self.span(parent_context_id);

        if let Some(parent) = parent {
            let _guard = parent.enter();
            emit_notification(parent_context_id, name);
        } else {
            emit_notification(parent_context_id, name);
        }
    }
}

impl TracingObserver {
    fn span(&self, id: RuntimeContextId) -> Option<Span> {
        self.spans
            .lock()
            .expect("tracing observer lock poisoned")
            .get(&id)
            .cloned()
    }
}

fn emit_notification(parent_context_id: RuntimeContextId, name: &'static str) {
    tracing::info!(
        perfetto = true,
        parent_context_id = parent_context_id.0,
        crux_name = name,
        "crux.notification"
    );
}
