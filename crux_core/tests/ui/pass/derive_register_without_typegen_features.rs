//! `register(..)` in a crate that declares neither `typegen` nor
//! `facet_typegen`.
//!
//! The generated type generation hooks are gated on those two feature names,
//! and a `cfg` naming a feature the crate does not declare is an
//! `unexpected_cfgs` warning — fatal in a crate built with `-D warnings`, as
//! every example in this repository is. The generated `impl` therefore carries
//! `#[allow(unexpected_cfgs)]`; the `deny` below asks for the lint back.
//!
//! Note that this case cannot fail on its own: trybuild's harness crate
//! mirrors `crux_core`'s feature list, so both names are declared there and
//! the lint has nothing to report either way. The guard against the
//! regression is the `register_extra_types` snapshot in `crux_macros`; what
//! this case checks is that a `register(..)` operation compiles and can be
//! sent, outside an `#[effect]` enum.

#![deny(unexpected_cfgs)]

use crux_core::{Command, macros::Operation};
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Facet, Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub enum StoreError {
    NotFound,
}

#[derive(Facet, Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub enum GetResult {
    Ok(Vec<u8>),
    Err(StoreError),
}

#[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
#[operation(request, output = GetResult, register(StoreError))]
pub struct Get {
    key: String,
}

pub enum Event {
    Got(GetResult),
}

fn main() {
    let _: Command<crux_core::Request<Get>, Event> = Command::request_from_shell(Get {
        key: "key".to_string(),
    })
    .then_send(Event::Got);
}
