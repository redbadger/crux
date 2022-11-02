use super::capability::Capability;
use crate::Request;
use derive_more::Deref;

#[derive(Deref)]
pub struct Time<Msg>(Capability<Msg, Option<bool>, String>);

impl<Msg> Default for Time<Msg> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<Msg> Time<Msg> {
    pub fn get<F>(&self, msg: F) -> Request
    where
        F: FnOnce(String) -> Msg + Sync + Send + 'static,
    {
        Request::Time {
            data: self.0.request(None, msg),
        }
    }
}
