use crate::{Continuations, Request, RequestBody};

pub struct Platform<'c, Msg> {
    continuations: &'c Continuations<Msg>,
}

impl<'c, Msg> Platform<'c, Msg> {
    pub fn new(continuations: &'c Continuations<Msg>) -> Self {
        Self { continuations }
    }

    pub fn get<F>(&self, msg: F) -> Request
    where
        F: FnOnce(String) -> Msg + Sync + Send + 'static,
    {
        let body = RequestBody::Platform;
        self.continuations
            .platform
            .write()
            .unwrap()
            .pause(body, msg)
    }
}
