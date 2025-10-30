#![deny(clippy::pedantic)]
//! A demo capability to get a name of the current platform

pub mod command;

#[expect(deprecated)]
use crux_core::capability::CapabilityContext;

use crux_core::capability::Operation;
use crux_core::macros::Capability;
use crux_core::{MaybeSend, MaybeSync};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformRequest;

// TODO revisit this
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformResponse(pub String);

impl Operation for PlatformRequest {
    type Output = PlatformResponse;
}

#[derive(Capability)]
#[deprecated(
    since = "0.7.0",
    note = "The Platform capability has been deprecated. Use command::Platform"
)]
#[expect(deprecated)]
pub struct Platform<Ev> {
    context: CapabilityContext<PlatformRequest, Ev>,
}

#[expect(deprecated)]
impl<Ev> Platform<Ev>
where
    Ev: 'static,
{
    #[must_use]
    pub fn new(context: CapabilityContext<PlatformRequest, Ev>) -> Self {
        Self { context }
    }

    pub fn get<F>(&self, callback: F)
    where
        F: FnOnce(PlatformResponse) -> Ev + MaybeSend + MaybeSync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let response = context.request_from_shell(PlatformRequest).await;

                context.update_app(callback(response));
            }
        });
    }
}
