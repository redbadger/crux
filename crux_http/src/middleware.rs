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
//! use futures_util::future::BoxFuture;
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

use async_trait::async_trait;
use futures_util::future::BoxFuture;

/// Middleware that wraps around remaining middleware chain.
#[async_trait]
pub trait Middleware: 'static + Send + Sync {
    /// Asynchronously handle the request, and return a response.
    async fn handle(&self, req: Request, client: Client, next: Next<'_>) -> Result<ResponseAsync>;
}

// This allows functions to work as middleware too.
#[async_trait]
impl<F> Middleware for F
where
    F: Send
        + Sync
        + 'static
        + for<'a> Fn(Request, Client, Next<'a>) -> BoxFuture<'a, Result<ResponseAsync>>,
{
    async fn handle(&self, req: Request, client: Client, next: Next<'_>) -> Result<ResponseAsync> {
        (self)(req, client, next).await
    }
}

/// The remainder of a middleware chain, including the endpoint.
#[allow(missing_debug_implementations)]
#[derive(Copy, Clone)]
pub struct Next<'a> {
    next_middleware: &'a [Arc<dyn Middleware>],
    endpoint: &'a (dyn (Fn(Request, Client) -> BoxFuture<'static, Result<ResponseAsync>>)
             + Send
             + Sync
             + 'static),
}

impl<'a> Next<'a> {
    /// Create a new instance
    pub fn new(
        next: &'a [Arc<dyn Middleware>],
        endpoint: &'a (dyn (Fn(Request, Client) -> BoxFuture<'static, Result<ResponseAsync>>)
                 + Send
                 + Sync
                 + 'static),
    ) -> Self {
        Self {
            endpoint,
            next_middleware: next,
        }
    }

    /// Asynchronously execute the remaining middleware chain.
    pub fn run(mut self, req: Request, client: Client) -> BoxFuture<'a, Result<ResponseAsync>> {
        if let Some((current, next)) = self.next_middleware.split_first() {
            self.next_middleware = next;
            current.handle(req, client, self)
        } else {
            (self.endpoint)(req, client)
        }
    }
}
