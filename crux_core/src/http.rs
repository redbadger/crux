//! TODO mod docs

use crate::Command;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct Request {
    method: String, // FIXME this probably should be an enum instead.
    url: String,
    // TODO support headers
}

#[derive(Debug, Deserialize)]
pub struct Response {
    status: u16,   // FIXME this probably should be a giant enum instead.
    body: Vec<u8>, // TODO support headers
}

pub struct Http<MakeEffect, Ef>
where
    MakeEffect: Fn(Request) -> Ef,
{
    effect: MakeEffect,
}

impl<MakeEffect, Ef> Http<MakeEffect, Ef>
where
    MakeEffect: Fn(Request) -> Ef,
{
    pub fn new(effect: MakeEffect) -> Self {
        Self { effect }
    }

    pub fn get<Ev, F>(&self, url: &str, callback: F) -> Command<Ef, Ev>
    where
        Ev: 'static,
        F: Fn(Response) -> Ev + 'static,
    {
        self.request("GET", url, callback)
    }

    pub fn request<Ev, F>(&self, method: &str, url: &str, callback: F) -> Command<Ef, Ev>
    where
        Ev: 'static,
        F: Fn(Response) -> Ev + 'static,
    {
        let request = Request {
            method: method.to_string(),
            url: url.to_string(),
        };

        Command::new((self.effect)(request), callback)
    }
}
