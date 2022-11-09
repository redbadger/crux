use crate::{continuations::ContinuationStore, Request, RequestBody};

pub struct Platform<Msg> {
    pub continuations: ContinuationStore<String, Msg>,
}

impl<Msg> Default for Platform<Msg> {
    fn default() -> Self {
        Self {
            continuations: Default::default(),
        }
    }
}

impl<Msg> Platform<Msg> {
    pub fn get<F>(&self, msg: F) -> (RequestBody, Msg)
    where
        F: FnOnce(String) -> Msg + Sync + Send + 'static,
    {
        let body = RequestBody::Platform;
        self.continuations.pause(body, msg)
    }
}
