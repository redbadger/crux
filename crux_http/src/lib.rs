#![deny(clippy::pedantic)]
//! A HTTP client for use with Crux
//!
//! `crux_http` allows Crux apps to make HTTP requests by asking the Shell to perform them.
//!
//! This is still work in progress and large parts of HTTP are not yet supported.
// #![warn(missing_docs)]

mod config;
mod error;
mod expect;
mod request;
mod request_builder;
mod response;

pub mod client;
pub mod command;
pub mod middleware;
pub mod protocol;
pub mod testing;

pub use http_types as http;

pub use self::{
    config::Config,
    error::HttpError,
    request::Request,
    request_builder::RequestBuilder,
    response::{Response, ResponseAsync},
};

use client::Client;

pub type Result<T> = std::result::Result<T, HttpError>;
