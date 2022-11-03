use crate::{Continuations, Request, RequestBody};

pub struct Http<'c, Msg> {
    continuations: &'c Continuations<Msg>,
}

impl<'c, Msg> Http<'c, Msg> {
    pub fn new(continuations: &'c Continuations<Msg>) -> Self {
        Self { continuations }
    }

    pub fn get<F>(&self, url: String, msg: F) -> Request
    where
        F: FnOnce(Vec<u8>) -> Msg + Sync + Send + 'static,
    {
        let body = RequestBody::Http(url);
        self.continuations.http.write().unwrap().pause(body, msg)
    }
}
