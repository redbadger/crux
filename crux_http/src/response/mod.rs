mod decode;
#[allow(clippy::module_inception)]
mod response;
mod response_async;

pub use self::{response::Response, response_async::ResponseAsync};

pub(crate) fn new_headers() -> crate::http::Headers {
    // http-types doesn't seem to let you construct a Headers, very annoying.
    // So here's a horrible hack to do it.
    crate::http::Request::new(crate::http::Method::Get, "https://thisisveryannoying.com")
        .as_ref()
        .clone()
}
