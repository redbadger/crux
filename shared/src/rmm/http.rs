use super::capability::Capability;
use crate::Request;
use derive_more::Deref;

#[derive(Deref)]
pub struct Http<Msg>(Capability<Msg, String, Vec<u8>>);

impl<Msg> Default for Http<Msg> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<Msg> Http<Msg> {
    pub fn get<F>(&self, url: String, msg: F) -> Request
    where
        F: FnOnce(Vec<u8>) -> Msg + Sync + Send + 'static,
    {
        Request::Http {
            data: self.0.request(url, msg),
        }
    }
}
