//! Middleware types
//!
//! # Examples
//! ```no_run
//! use crux_http::middleware::{Next, Middleware};
//! use crux_http::{client::Client, Request, ResponseAsync, Result};
//! use std::time;
//! use std::sync::Arc;
//!
//! /// Log each request's duration
//! #[derive(Debug)]
//! pub struct Logger;
//!
//! #[async_trait::async_trait]
//! impl Middleware for Logger {
//!     async fn handle(
//!         &self,
//!         req: Request,
//!         client: Client,
//!         next: Next<'_>,
//!     ) -> Result<ResponseAsync> {
//!         println!("sending request to {}", req.url());
//!         let now = time::Instant::now();
//!         let res = next.run(req, client).await?;
//!         println!("request completed ({:?})", now.elapsed());
//!         Ok(res)
//!     }
//! }
//! ```
//! `Middleware` can also be instantiated using a free function thanks to some convenient trait
//! implementations.
//!
//! ```no_run
//! use crux_core::BoxFuture;
//! use crux_http::middleware::{Next, Middleware};
//! use crux_http::{client::Client, Request, ResponseAsync, Result};
//! use std::time;
//! use std::sync::Arc;
//!
//! fn logger<'a>(req: Request, client: Client, next: Next<'a>) -> BoxFuture<'a, Result<ResponseAsync>> {
//!     Box::pin(async move {
//!         println!("sending request to {}", req.url());
//!         let now = time::Instant::now();
//!         let res = next.run(req, client).await?;
//!         println!("request completed ({:?})", now.elapsed());
//!         Ok(res)
//!     })
//! }
//! ```

use std::sync::Arc;

use crate::{Client, Request, ResponseAsync, Result};

mod redirect;

pub use redirect::Redirect;

use crux_core::{BoxFuture, MaybeSend, MaybeSync};

/// Middleware that wraps around remaining middleware chain.
pub trait Middleware: 'static + MaybeSend + MaybeSync {
    /// Asynchronously handle the request, and return a response.
    fn handle<'a>(
        &'a self,
        req: Request,
        client: Client,
        next: Next<'a>,
    ) -> BoxFuture<'a, Result<ResponseAsync>>;
}

// This allows functions to work as middleware too.
impl<F> Middleware for F
where
    F: MaybeSend
        + MaybeSync
        + 'static
        + for<'a> Fn(Request, Client, Next<'a>) -> BoxFuture<'a, Result<ResponseAsync>>,
{
    fn handle<'a>(
        &'a self,
        req: Request,
        client: Client,
        next: Next<'a>,
    ) -> BoxFuture<'a, Result<ResponseAsync>> {
        (self)(req, client, next)
    }
}

pub trait Endpoint:
    Fn(Request, Client) -> BoxFuture<'static, Result<ResponseAsync>> + MaybeSend + MaybeSync + 'static
{
}

impl<T> Endpoint for T where
    T: Fn(Request, Client) -> BoxFuture<'static, Result<ResponseAsync>>
        + MaybeSend
        + MaybeSync
        + 'static
{
}

/// The remainder of a middleware chain, including the endpoint.
#[allow(missing_debug_implementations)]
#[derive(Copy, Clone)]
pub struct Next<'a> {
    next_middleware: &'a [Arc<dyn Middleware>],
    endpoint: &'a dyn Endpoint,
}

impl<'a> Next<'a> {
    /// Create a new instance
    pub fn new(next: &'a [Arc<dyn Middleware>], endpoint: &'a dyn Endpoint) -> Self {
        Self {
            endpoint,
            next_middleware: next,
        }
    }

    /// Asynchronously execute the remaining middleware chain.
    #[must_use]
    pub fn run(mut self, req: Request, client: Client) -> BoxFuture<'a, Result<ResponseAsync>> {
        if let Some((current, next)) = self.next_middleware.split_first() {
            self.next_middleware = next;
            current.handle(req, client, self)
        } else {
            (self.endpoint)(req, client)
        }
    }
}
