use super::capability::Capability;
use crate::Request;
use derive_more::Deref;

#[derive(Deref)]
pub struct Platform<Msg>(Capability<Msg, Option<bool>, String>);

impl<Msg> Default for Platform<Msg> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<Msg> Platform<Msg> {
    pub fn get<F>(&self, msg: F) -> Request
    where
        F: FnOnce(String) -> Msg + Sync + Send + 'static,
    {
        Request::Platform {
            data: self.0.request(None, msg),
        }
    }
}
