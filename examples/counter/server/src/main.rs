use futures_signals::signal::{Mutable, SignalExt};
use std::{convert::Infallible, net::SocketAddr};

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
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct CounterState {
    value: Mutable<isize>,
}

#[derive(Serialize)]
struct Counter {
    value: isize,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_counter=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = CounterState {
        value: Mutable::new(0),
    };

    let app = Router::new()
        .route("/", get(get_counter))
        .route("/sse", get(sse_handler))
        .route("/inc", post(inc))
        .route("/dec", post(dec))
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST])
                .allow_origin(Any),
        )
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_counter(State(counter): State<CounterState>) -> impl IntoResponse {
    let value = *counter.value.lock_mut();

    Json(Counter { value })
}

async fn inc(State(counter): State<CounterState>) -> impl IntoResponse {
    let value = {
        let mut value = counter.value.lock_mut();
        *value = value.saturating_add(1);
        *value
    };

    Json(Counter { value })
}

async fn dec(State(counter): State<CounterState>) -> impl IntoResponse {
    let value = {
        let mut value = counter.value.lock_mut();
        *value = value.saturating_sub(1);
        *value
    };

    Json(Counter { value })
}

async fn sse_handler(
    State(counter): State<CounterState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = counter
        .value
        .signal()
        .map(|value| Event::default().data(&serde_json::to_string(&Counter { value }).unwrap()))
        .map(Ok)
        .to_stream();

    Sse::new(stream).keep_alive(KeepAlive::default())
}
