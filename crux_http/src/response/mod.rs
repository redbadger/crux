mod decode;
#[allow(clippy::module_inception)]
mod response;
mod response_async;

pub use self::{response::Response, response_async::ResponseAsync};

pub(crate) fn new_headers() -> http_types::Headers {
    // http-types doesn't seem to let you construct a Headers, very annoying.
    // So here's a horrible hack to do it.
    http_types::Request::new(http_types::Method::Get, "https://thisisveryannoying.com")
        .as_ref()
        .clone()
}
