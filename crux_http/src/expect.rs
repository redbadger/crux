use std::marker::PhantomData;

use http_types::convert::DeserializeOwned;

use crate::{Response, Result};

pub trait ResponseExpectation {
    type Body;

    fn decode(&self, resp: crate::Response<Vec<u8>>) -> Result<Response<Self::Body>>;
}

pub struct ExpectBytes;

impl ResponseExpectation for ExpectBytes {
    type Body = Vec<u8>;

    fn decode(&self, resp: crate::Response<Vec<u8>>) -> Result<Response<Vec<u8>>> {
        Ok(resp)
    }
}

#[derive(Default)]
pub struct ExpectString;

impl ResponseExpectation for ExpectString {
    type Body = String;

    fn decode(&self, mut resp: crate::Response<Vec<u8>>) -> Result<Response<String>> {
        let body = resp.body_string()?;
        Ok(resp.with_body(body))
    }
}

pub struct ExpectJson<T> {
    phantom: PhantomData<fn() -> T>,
}

impl<T> Default for ExpectJson<T> {
    fn default() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl<T> ResponseExpectation for ExpectJson<T>
where
    T: DeserializeOwned,
{
    type Body = T;

    fn decode(&self, mut resp: crate::Response<Vec<u8>>) -> Result<Response<T>> {
        let body = resp.body_json::<T>()?;
        Ok(resp.with_body(body))
    }
}
