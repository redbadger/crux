use crux_http::{HttpError, Response, Result};
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Facet, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum HttpResult<T> {
    Ok(T),
    Err(HttpError),
}

impl<T> From<Result<Response<T>>> for HttpResult<Response<T>> {
    fn from(value: Result<Response<T>>) -> Self {
        match value {
            Ok(response) => HttpResult::Ok(response),
            Err(error) => HttpResult::Err(error),
        }
    }
}
