use axum::{
    Json,
    extract::State,
    response::{
        IntoResponse, Sse,
        sse::{Event, KeepAlive},
    },
};
use futures::Stream;
use futures_signals::signal::{Mutable, SignalExt};
use serde::Serialize;
use std::convert::Infallible;

use crate::now;

#[derive(Clone)]
pub struct CounterState {
    pub value: Mutable<Counter>,
}

#[derive(Serialize, Copy, Clone)]
pub struct Counter {
    pub value: isize,
    pub updated_at: u128,
}

pub async fn get_counter(State(counter): State<CounterState>) -> impl IntoResponse {
    let value = counter.value.lock_mut();

    Json(*value)
}

pub async fn inc(State(counter): State<CounterState>) -> impl IntoResponse {
    let mut value = counter.value.lock_mut();
    let new = value.value.saturating_add(1);
    *value = Counter {
        value: new,
        updated_at: now(),
    };

    Json(*value)
}

pub async fn dec(State(counter): State<CounterState>) -> impl IntoResponse {
    let mut value = counter.value.lock_mut();
    let new = value.value.saturating_sub(1);
    *value = Counter {
        value: new,
        updated_at: now(),
    };

    Json(*value)
}

pub async fn sse_handler(
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
