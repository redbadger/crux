//! A small, buffered tracing exporter for this demo, using Perfetto's Chrome
//! JSON import. Each span has its own track so overlapping command/effect
//! lifetimes cannot be mistaken for a stack of nested synchronous calls.

use std::{
    collections::HashMap,
    fs::File,
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};

use serde_json::{Map, Value, json};
use tracing::{
    Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id},
};
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

#[derive(Clone)]
pub struct TraceLayer {
    clock: Instant,
    records: Arc<Mutex<Records>>,
}

#[derive(Default)]
struct Records {
    spans: Vec<SpanRecord>,
    events: Vec<Value>,
}

struct SpanRecord {
    start: f64,
    end: f64,
    category: &'static str,
    fields: Fields,
}

#[derive(Default)]
struct Fields(Map<String, Value>);

impl Fields {
    fn id(&self, key: &str) -> u64 {
        self.0.get(key).and_then(Value::as_u64).unwrap_or(0)
    }
    fn name(&self) -> &str {
        self.0
            .get("crux_name")
            .and_then(Value::as_str)
            .unwrap_or("crux")
    }
}

impl Visit for Fields {
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name().into(), value.into());
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().into(), value.into());
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().into(), format!("{value:?}").into());
    }
}

impl TraceLayer {
    pub fn new() -> Self {
        Self {
            clock: Instant::now(),
            records: Arc::default(),
        }
    }

    fn now(&self) -> f64 {
        self.clock.elapsed().as_secs_f64() * 1_000_000.0
    }

    pub fn write(&self, path: &Path) -> anyhow::Result<()> {
        serde_json::to_writer(File::create(path)?, &self.document())?;
        Ok(())
    }

    fn document(&self) -> Value {
        let records = self.records.lock().expect("trace lock poisoned");
        let spans: HashMap<_, _> = records
            .spans
            .iter()
            .map(|span| (span.fields.id("context_id"), span))
            .collect();
        let mut events = records.events.clone();
        for span in &records.spans {
            let id = span.fields.id("context_id");
            events.push(
                json!({"ph": "M", "name": "thread_name", "pid": 1, "tid": id,
                "args": {"name": format!("{}: {} ({id})", span.category, span.fields.name())}}),
            );
            events.push(json!({"ph": "X", "name": span.fields.name(), "cat": span.category,
                "pid": 1, "tid": id, "ts": span.start, "dur": span.end - span.start, "args": span.fields.0}));
            let cause = span.fields.id("caused_by");
            if let Some(source) = spans.get(&cause) {
                // This arrow denotes a span-level causal relationship, not an
                // event emission timestamp. Anchor it at the source span's
                // start: later instants on that track would otherwise become
                // the flow source in the Chrome JSON importer.
                let from = source.start;
                events.push(
                    json!({"ph": "s", "name": "caused by", "cat": "causality", "id": id,
                    "pid": 1, "tid": cause, "ts": from}),
                );
                events.push(
                    json!({"ph": "f", "name": "caused by", "cat": "causality", "id": id,
                    "pid": 1, "tid": id, "ts": span.start, "bp": "e"}),
                );
            }
        }
        drop(records);
        events.sort_by(|a, b| {
            a["ts"]
                .as_f64()
                .unwrap_or(0.0)
                .total_cmp(&b["ts"].as_f64().unwrap_or(0.0))
        });
        json!({"traceEvents": events, "displayTimeUnit": "ms"})
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for TraceLayer {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let category = attrs.metadata().name();
        if !category.starts_with("crux.") {
            return;
        }
        let mut fields = Fields::default();
        attrs.record(&mut fields);
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(SpanRecord {
                start: self.now(),
                end: 0.0,
                category,
                fields,
            });
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let record = ctx
            .span(&id)
            .and_then(|span| span.extensions_mut().remove::<SpanRecord>());
        if let Some(mut record) = record {
            record.end = self.now();
            self.records
                .lock()
                .expect("trace lock poisoned")
                .spans
                .push(record);
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
        let category = event.metadata().name();
        if !category.starts_with("crux.") {
            return;
        }
        let mut fields = Fields::default();
        event.record(&mut fields);
        self.records
            .lock()
            .expect("trace lock poisoned")
            .events
            .push(json!({
                "ph": "I", "s": "t", "name": fields.name(), "cat": category,
                "pid": 1, "tid": fields.id("context_id"), "ts": self.now(), "args": fields.0
            }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::prelude::*;

    #[test]
    fn preserves_closed_causes_and_overlapping_span_lifetimes() {
        let trace = TraceLayer::new();
        let subscriber = tracing_subscriber::registry().with(trace.clone());
        tracing::subscriber::with_default(subscriber, || {
            let first = tracing::info_span!(parent: None, "crux.event", context_id = 1_u64, caused_by = 0_u64, crux_name = "First");
            tracing::info!(name: "crux.notification", context_id = 1_u64, crux_name = "Render");
            drop(first);
            let second = tracing::info_span!(parent: None, "crux.event", context_id = 2_u64, caused_by = 1_u64, crux_name = "Second");
            let third = tracing::info_span!(parent: None, "crux.event", context_id = 3_u64, caused_by = 2_u64, crux_name = "Third");
            drop(second);
            tracing::info!(name: "crux.stream.item", context_id = 3_u64, crux_name = "response");
            drop(third);
        });
        let doc = trace.document();
        let events = doc["traceEvents"].as_array().unwrap();
        let slices: HashMap<_, _> = events
            .iter()
            .filter(|e| e["ph"] == "X")
            .map(|e| (e["tid"].as_u64().unwrap(), e))
            .collect();
        let end = |id| slices[&id]["ts"].as_f64().unwrap() + slices[&id]["dur"].as_f64().unwrap();
        assert!(end(1) <= slices[&2]["ts"].as_f64().unwrap());
        assert!(slices[&3]["ts"].as_f64().unwrap() <= end(2));
        assert!(end(2) <= end(3));
        let source = events
            .iter()
            .find(|e| e["ph"] == "s" && e["tid"] == 1)
            .unwrap();
        assert_eq!(
            source["ts"], slices[&1]["ts"],
            "flow must attach to the command, not its notification"
        );
        assert_eq!(events.iter().filter(|e| e["ph"] == "s").count(), 2);
        assert_eq!(events.iter().filter(|e| e["ph"] == "f").count(), 2);
        assert!(events.iter().any(|e| e["ph"] == "I" && e["tid"] == 3));
    }
}
