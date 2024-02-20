use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::protocol::{EffectSender, HttpRequest, HttpResponse, HttpResult};

/// FakeShell implements EffectSender for use in our internal tests.
#[derive(Clone, Default)]
pub(crate) struct FakeShell {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    requests_received: Vec<HttpRequest>,
    responses_to_provide: VecDeque<HttpResponse>,
}

impl FakeShell {
    pub fn provide_response(&mut self, response: HttpResponse) {
        self.inner
            .lock()
            .unwrap()
            .responses_to_provide
            .push_front(response);
    }

    pub fn take_requests_received(&mut self) -> Vec<HttpRequest> {
        let mut inner = self.inner.lock().unwrap();
        std::mem::take(&mut inner.requests_received)
    }
}

#[async_trait]
impl EffectSender for FakeShell {
    async fn send(&self, effect: HttpRequest) -> HttpResult {
        let mut inner = self.inner.lock().unwrap();
        inner.requests_received.push(effect);

        HttpResult::Ok(
            inner
                .responses_to_provide
                .pop_back()
                .expect("test tried to send an unexpected HttpRequest"),
        )
    }
}
