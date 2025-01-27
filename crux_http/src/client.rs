use std::fmt;
use std::sync::Arc;

use crate::middleware::{Middleware, Next};
use crate::protocol::{EffectSender, HttpResult, ProtocolRequestBuilder};
use crate::{Config, Request, RequestBuilder, ResponseAsync, Result};
use http_types::{Method, Url};

/// An HTTP client, capable of sending `Request`s
///
/// Users should only interact with this type from middlewares - normal crux code should
/// make use of the `Http` capability type instead.
///
/// # Examples
///
/// ```no_run
/// use futures_util::future::BoxFuture;
/// use crux_http::middleware::{Next, Middleware};
/// use crux_http::{client::Client, Request, RequestBuilder, ResponseAsync, Result};
/// use std::time;
/// use std::sync::Arc;
///
/// // Fetches an authorization token prior to making a request
/// fn fetch_auth<'a>(mut req: Request, client: Client, next: Next<'a>) -> BoxFuture<'a, Result<ResponseAsync>> {
///     Box::pin(async move {
///         let auth_token = client.get("https://httpbin.org/get")
///             .await?
///             .body_string()
///             .await?;
///         req.append_header("Authorization", format!("Bearer {auth_token}"));
///         next.run(req, client).await
///     })
/// }
/// ```
pub struct Client {
    config: Config,
    effect_sender: Arc<dyn EffectSender + Send + Sync>,
    /// Holds the middleware stack.
    ///
    /// Note(Fishrock123): We do actually want this structure.
    /// The outer Arc allows us to clone in .send() without cloning the array.
    /// The Vec allows us to add middleware at runtime.
    /// The inner Arc-s allow us to implement Clone without sharing the vector with the parent.
    /// We don't use a Mutex around the Vec here because adding a middleware during execution should be an error.
    #[allow(clippy::rc_buffer)]
    middleware: Arc<Vec<Arc<dyn Middleware>>>,
}

impl Clone for Client {
    /// Clones the Client.
    ///
    /// This copies the middleware stack from the original, but shares
    /// the `HttpClient` and http client config of the original.
    /// Note that individual middleware in the middleware stack are
    /// still shared by reference.
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            effect_sender: Arc::clone(&self.effect_sender),
            middleware: Arc::new(self.middleware.iter().cloned().collect()),
        }
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Client {{}}")
    }
}

impl Client {
    pub(crate) fn new<Sender>(sender: Sender) -> Self
    where
        Sender: EffectSender + Send + Sync + 'static,
    {
        Self {
            config: Config::default(),
            effect_sender: Arc::new(sender),
            middleware: Arc::new(vec![]),
        }
    }

    // This is currently dead code because there's no easy way to configure a client.
    // TODO: fix that in some future PR
    #[allow(dead_code)]
    /// Push middleware onto the middleware stack.
    ///
    /// See the [middleware] submodule for more information on middleware.
    ///
    /// [middleware]: ../middleware/index.html
    pub(crate) fn with(mut self, middleware: impl Middleware) -> Self {
        let m = Arc::get_mut(&mut self.middleware)
            .expect("Registering middleware is not possible after the Client has been used");
        m.push(Arc::new(middleware));
        self
    }

    /// Send a `Request` using this client.
    pub async fn send(&self, req: impl Into<Request>) -> Result<ResponseAsync> {
        let mut req: Request = req.into();
        let middleware = self.middleware.clone();

        let mw_stack = match req.take_middleware() {
            Some(req_mw) => {
                let mut mw = Vec::with_capacity(middleware.len() + req_mw.len());
                mw.extend(middleware.iter().cloned());
                mw.extend(req_mw);
                Arc::new(mw)
            }
            None => middleware,
        };

        let next = Next::new(&mw_stack, &|req, client| {
            Box::pin(async move {
                let req = req
                    .into_protocol_request()
                    .await
                    .expect("Failed to create request");
                match client.effect_sender.send(req).await {
                    HttpResult::Ok(res) => Ok(res.into()),
                    HttpResult::Err(e) => Err(e),
                }
            })
        });

        let client = Self {
            config: self.config.clone(),
            effect_sender: Arc::clone(&self.effect_sender),
            // Erase the middleware stack for the Client accessible from within middleware.
            // This avoids gratuitous circular borrow & logic issues.
            middleware: Arc::new(vec![]),
        };

        let res = next.run(req, client).await?;
        Ok(ResponseAsync::new(res.into()))
    }

    /// Submit a `Request` and get the response body as bytes.
    pub async fn recv_bytes(&self, req: impl Into<Request>) -> Result<Vec<u8>> {
        let mut res = self.send(req.into()).await?;
        res.body_bytes().await
    }

    /// Submit a `Request` and get the response body as a string.
    pub async fn recv_string(&self, req: impl Into<Request>) -> Result<String> {
        let mut res = self.send(req.into()).await?;
        res.body_string().await
    }

    /// Submit a `Request` and decode the response body from json into a struct.
    pub async fn recv_json<T: serde::de::DeserializeOwned>(
        &self,
        req: impl Into<Request>,
    ) -> Result<T> {
        let mut res = self.send(req.into()).await?;
        res.body_json::<T>().await
    }

    /// Submit a `Request` and decode the response body from form encoding into a struct.
    ///
    /// # Errors
    ///
    /// Any I/O error encountered while reading the body is immediately returned
    /// as an `Err`.
    ///
    /// If the body cannot be interpreted as valid json for the target type `T`,
    /// an `Err` is returned.
    pub async fn recv_form<T: serde::de::DeserializeOwned>(
        &self,
        req: impl Into<Request>,
    ) -> Result<T> {
        let mut res = self.send(req.into()).await?;
        res.body_form::<T>().await
    }

    /// Perform an HTTP `GET` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    pub fn get(&self, uri: impl AsRef<str>) -> RequestBuilder<()> {
        RequestBuilder::new_for_middleware(Method::Get, self.url(uri), self.clone())
    }

    /// Perform an HTTP `HEAD` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    pub fn head(&self, uri: impl AsRef<str>) -> RequestBuilder<()> {
        RequestBuilder::new_for_middleware(Method::Head, self.url(uri), self.clone())
    }

    /// Perform an HTTP `POST` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    pub fn post(&self, uri: impl AsRef<str>) -> RequestBuilder<()> {
        RequestBuilder::new_for_middleware(Method::Post, self.url(uri), self.clone())
    }

    /// Perform an HTTP `PUT` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    pub fn put(&self, uri: impl AsRef<str>) -> RequestBuilder<()> {
        RequestBuilder::new_for_middleware(Method::Put, self.url(uri), self.clone())
    }

    /// Perform an HTTP `DELETE` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    pub fn delete(&self, uri: impl AsRef<str>) -> RequestBuilder<()> {
        RequestBuilder::new_for_middleware(Method::Delete, self.url(uri), self.clone())
    }

    /// Perform an HTTP `CONNECT` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    pub fn connect(&self, uri: impl AsRef<str>) -> RequestBuilder<()> {
        RequestBuilder::new_for_middleware(Method::Connect, self.url(uri), self.clone())
    }

    /// Perform an HTTP `OPTIONS` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    pub fn options(&self, uri: impl AsRef<str>) -> RequestBuilder<()> {
        RequestBuilder::new_for_middleware(Method::Options, self.url(uri), self.clone())
    }

    /// Perform an HTTP `TRACE` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    pub fn trace(&self, uri: impl AsRef<str>) -> RequestBuilder<()> {
        RequestBuilder::new_for_middleware(Method::Trace, self.url(uri), self.clone())
    }

    /// Perform an HTTP `PATCH` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    pub fn patch(&self, uri: impl AsRef<str>) -> RequestBuilder<()> {
        RequestBuilder::new_for_middleware(Method::Patch, self.url(uri), self.clone())
    }

    /// Perform a HTTP request with the given verb using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    pub fn request(&self, verb: Method, uri: impl AsRef<str>) -> RequestBuilder<()> {
        RequestBuilder::new_for_middleware(verb, self.url(uri), self.clone())
    }

    /// Get the current configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    // private function to generate a url based on the base_path
    fn url(&self, uri: impl AsRef<str>) -> Url {
        match &self.config.base_url {
            None => uri.as_ref().parse().unwrap(),
            Some(base) => base.join(uri.as_ref()).unwrap(),
        }
    }
}

#[cfg(test)]
mod client_tests {
    use super::Client;
    use crate::protocol::{HttpRequest, HttpResponse};
    use crate::testing::FakeShell;

    #[futures_test::test]
    async fn an_http_get() {
        let mut shell = FakeShell::default();
        shell.provide_response(HttpResponse::ok().body("Hello World!").build());

        let client = Client::new(shell.clone());

        let mut response = client.get("https://example.com").await.unwrap();
        assert_eq!(response.body_string().await.unwrap(), "Hello World!");

        assert_eq!(
            shell.take_requests_received(),
            vec![HttpRequest::get("https://example.com/").build()]
        )
    }
}
