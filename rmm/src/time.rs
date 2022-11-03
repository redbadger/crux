use crate::{continuations::ContinuationStore, Request, RequestBody};

pub struct Time<Msg> {
    pub continuations: ContinuationStore<String, Msg>,
}

impl<Msg> Default for Time<Msg> {
    fn default() -> Self {
        Self {
            continuations: Default::default(),
        }
    }
}

impl<Msg> Time<Msg> {
    pub fn get<F>(&mut self, msg: F) -> Request
    where
        F: FnOnce(String) -> Msg + Sync + Send + 'static,
    {
        let body = RequestBody::Time;
        self.continuations.pause(body, msg)
    }
}
