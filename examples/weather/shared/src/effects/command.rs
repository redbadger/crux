use crux_core::{command::RequestBuilder, Command, Request};
use std::{future::Future, marker::PhantomData};

use super::{LocationOperation, LocationResponse, LocationResult};

pub struct Location<Effect, Event> {
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

impl<Effect, Event> Location<Effect, Event>
where
    Effect: Send + From<Request<LocationOperation>> + 'static,
    Event: Send + 'static,
{
    #[must_use]
    pub fn is_location_enabled() -> RequestBuilder<Effect, Event, impl Future<Output = bool>> {
        Command::request_from_shell(LocationOperation::IsLocationEnabled).map(|result| match result
        {
            LocationResult::Enabled(val) => val,
            LocationResult::Location(_) => false,
        })
    }

    #[must_use]
    pub fn get_location(
    ) -> RequestBuilder<Effect, Event, impl Future<Output = Option<LocationResponse>>> {
        Command::request_from_shell(LocationOperation::GetLocation).map(|result| match result {
            LocationResult::Location(loc) => loc,
            LocationResult::Enabled(_) => None,
        })
    }
}
