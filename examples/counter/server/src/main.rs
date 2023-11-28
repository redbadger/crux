mod cats;

use axum::{
    extract::State,
    http::Method,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Sse,
    },
    routing::{get, post},
    Json, Router,
};
use futures::Stream;
use futures_signals::signal::{Mutable, SignalExt};
use serde::Serialize;
use std::{
    convert::Infallible,
    net::SocketAddr,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct CounterState {
    value: Mutable<Counter>,
}

#[derive(Serialize, Copy, Clone)]
struct Counter {
    value: isize,
    updated_at: u128,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = CounterState {
        value: Mutable::new(Counter {
            value: 0,
            updated_at: now(),
        }),
    };

    let app = Router::new()
        .route("/", get(get_counter))
        .route("/sse", get(sse_handler))
        .route("/inc", post(inc))
        .route("/dec", post(dec))
        .route("/cat", get(cats::get))
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST])
                .allow_origin(Any),
        )
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = TcpListener::bind(&addr).await.unwrap();

    tracing::info!("listening on {}", addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn get_counter(State(counter): State<CounterState>) -> impl IntoResponse {
    let value = counter.value.lock_mut();

    Json(*value)
}

async fn inc(State(counter): State<CounterState>) -> impl IntoResponse {
    let mut value = counter.value.lock_mut();
    let new = value.value.saturating_add(1);
    *value = Counter {
        value: new,
        updated_at: now(),
    };

    Json(*value)
}

async fn dec(State(counter): State<CounterState>) -> impl IntoResponse {
    let mut value = counter.value.lock_mut();
    let new = value.value.saturating_sub(1);
    *value = Counter {
        value: new,
        updated_at: now(),
    };

    Json(*value)
}

async fn sse_handler(
    State(counter): State<CounterState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = counter
        .value
        .signal()
        .map(|value| {
            let data = serde_json::to_string(&value).unwrap();
            Ok(Event::default().data(data))
        })
        .to_stream();

    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
