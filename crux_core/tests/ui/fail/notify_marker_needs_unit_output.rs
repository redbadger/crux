//! The `Notify` marker is only for operations the shell never answers, so its
//! output has to be `()`.

use crux_core::{RequestKind, capability::Operation, operation};

struct ANotification;

impl Operation for ANotification {
    type Output = usize;
    const KIND: Option<RequestKind> = Some(RequestKind::Notify);
}

impl operation::Notify for ANotification {}

fn main() {}
