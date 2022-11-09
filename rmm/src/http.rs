use crate::{continuations::ContinuationStore, Request, RequestBody};

pub struct Http<Msg> {
    pub continuations: ContinuationStore<Vec<u8>, Msg>,
}

impl<Msg> Default for Http<Msg> {
    fn default() -> Self {
        Self {
            continuations: Default::default(),
        }
    }
}

impl<Msg> Http<Msg> {
    pub fn get<F>(&self, url: String, msg: F) -> Request
    where
        F: FnOnce(Vec<u8>) -> Msg + Sync + Send + 'static,
    {
        let body = RequestBody::Http(url);
        self.continuations.pause(body, msg)
    }
}
