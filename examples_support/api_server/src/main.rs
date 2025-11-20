mod cats;
mod counter;

use axum::{
    Router,
    http::Method,
    routing::{get, post},
};
use futures_signals::signal::Mutable;
use std::{
    net::SocketAddr,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = counter::CounterState {
        value: Mutable::new(counter::Counter {
            value: 0,
            updated_at: now(),
        }),
    };

    let app = Router::new()
        .route("/", get(counter::get_counter))
        .route("/sse", get(counter::sse_handler))
        .route("/inc", post(counter::inc))
        .route("/dec", post(counter::dec))
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

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
