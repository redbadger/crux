use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct CounterState {
    value: Arc<Mutex<usize>>,
}

#[derive(Serialize)]
struct Counter {
    value: usize,
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
        value: Arc::new(Mutex::new(0)),
    };

    let app = Router::new().route("/", get(get_counter)).with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_counter(State(counter): State<CounterState>) -> impl IntoResponse {
    let value = {
        let mut value = counter.value.lock().unwrap();
        *value += 1;
        *value
    };

    Json(Counter { value })
}
