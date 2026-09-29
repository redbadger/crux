use crux_core::capability::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

pub mod sse;

// simple Random number capability without an API

// ANCHOR: operation
#[derive(Facet, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomNumberRequest(pub isize, pub isize); // request a random number from 1 to N, inclusive

#[derive(Facet, Debug, PartialEq, Eq, Deserialize)]
pub struct RandomNumber(pub isize);

impl Operation for RandomNumberRequest {
    type Output = RandomNumber;
}
// ANCHOR_END: operation
