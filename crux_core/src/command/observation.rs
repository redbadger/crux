//! Delivers a span guard from Core to the command's waiting side.

use std::sync::Arc;

use futures::{Future, FutureExt as _, Stream, StreamExt as _, channel::oneshot};
use std::task::Poll;

use crate::{CoreRuntimeObserver, RuntimeContextId};

/// Ends an effect span when the command stops waiting, independently of the shell.
pub struct SpanGuard {
    pub(crate) id: RuntimeContextId,
    pub(crate) observer: Arc<dyn CoreRuntimeObserver + Send + Sync>,
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        self.observer.effect_ended(self.id);
    }
}

/// Own the receiver from construction, and release it on completion or drop.
pub(super) fn observe_request<F: Future + Unpin>(
    request: F,
    receiver: oneshot::Receiver<SpanGuard>,
) -> impl Future<Output = F::Output> {
    // Drop the request before its guard, including before the first poll.
    let mut owned = Some((request, receiver));
    futures::future::poll_fn(move |cx| {
        let Some((future, _)) = owned.as_mut() else {
            return Poll::Pending;
        };
        let output = std::task::ready!(future.poll_unpin(cx));
        drop(owned.take());
        Poll::Ready(output)
    })
}

/// Observe consumed items, and end the span on stream termination or drop.
pub(super) fn observe_stream<S: Stream + Unpin>(
    stream: S,
    receiver: oneshot::Receiver<SpanGuard>,
) -> impl Stream<Item = S::Item> {
    // Drop the stream before the guard, whether it is buffered or received.
    let mut owned = Some((stream, receiver, None::<SpanGuard>));
    futures::stream::poll_fn(move |cx| {
        let Some((inner, receiver, guard)) = owned.as_mut() else {
            return Poll::Ready(None);
        };
        let output = std::task::ready!(inner.poll_next_unpin(cx));
        if output.is_some() {
            if guard.is_none() {
                *guard = receiver.try_recv().ok().flatten();
            }
            if let Some(guard) = guard {
                guard.observer.stream_item(guard.id);
            }
        } else {
            drop(owned.take());
        }
        Poll::Ready(output)
    })
}
