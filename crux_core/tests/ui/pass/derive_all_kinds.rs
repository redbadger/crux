//! `#[derive(Operation)]` in all three forms, each sent with the constructor
//! its declared kind allows, and gathered into an effect enum.

use crux_core::{
    Command,
    macros::{Operation, effect},
};
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
#[operation(notify)]
pub struct Publish(Vec<u8>);

#[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
#[operation(request, output = GetResult, register(StoreError))]
pub struct Get {
    key: String,
}

#[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
#[operation(stream, output = Vec<u8>)]
pub struct Subscribe;

#[derive(Facet, Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub enum GetResult {
    Ok(Vec<u8>),
    Err(StoreError),
}

#[derive(Facet, Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub enum StoreError {
    NotFound,
}

#[effect(facet_typegen)]
pub enum Effect {
    Publish(Publish),
    Get(Get),
    Subscribe(Subscribe),
}

pub enum Event {
    Published,
    Got(GetResult),
    Message(Vec<u8>),
}

fn main() {
    let _: Command<Effect, Event> = Command::notify_shell(Publish(vec![])).into();

    let _: Command<Effect, Event> = Command::request_from_shell(Get {
        key: "key".to_string(),
    })
    .then_send(Event::Got);

    let _: Command<Effect, Event> =
        Command::stream_from_shell(Subscribe).then_send(Event::Message);

    let _ = Event::Published;
}
