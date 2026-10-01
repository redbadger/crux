use crux_core::macros::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

pub mod sse;

// simple Random number capability without an API

// ANCHOR: operation
/// Request a random number between the two values, both included. The answer
/// is a single [`RandomNumber`].
#[derive(Operation, Facet, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[operation(request, output = RandomNumber)]
pub struct RandomNumberRequest(pub isize, pub isize);

#[derive(Facet, Debug, PartialEq, Eq, Deserialize)]
pub struct RandomNumber(pub isize);
// ANCHOR_END: operation
