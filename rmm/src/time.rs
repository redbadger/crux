use crate::{Continuations, Request, RequestBody};

pub struct Time<'c, Msg> {
    continuations: &'c Continuations<Msg>,
}

impl<'c, Msg> Time<'c, Msg> {
    pub fn new(continuations: &'c Continuations<Msg>) -> Self {
        Self { continuations }
    }

    pub fn get<F>(&self, msg: F) -> Request
    where
        F: FnOnce(String) -> Msg + Sync + Send + 'static,
    {
        let body = RequestBody::Time;
        self.continuations.time.write().unwrap().pause(body, msg)
    }
}
