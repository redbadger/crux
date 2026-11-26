//! Track the waiting side until completion or cancellation. Core checks
//! the sender before dispatch; no values need to be sent through these channels.
use futures::{Future, FutureExt as _, Stream, StreamExt as _, channel::oneshot};
use std::task::Poll;

pub(super) fn track_request<F: Future + Unpin>(
    request: F,
    receiver: oneshot::Receiver<()>,
) -> impl Future<Output = F::Output> {
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

pub(super) fn track_stream<S: Stream + Unpin>(
    stream: S,
    receiver: oneshot::Receiver<()>,
) -> impl Stream<Item = S::Item> {
    let mut owned = Some((stream, receiver));
    futures::stream::poll_fn(move |cx| {
        let Some((inner, _)) = owned.as_mut() else {
            return Poll::Ready(None);
        };
        let output = std::task::ready!(inner.poll_next_unpin(cx));
        if output.is_none() {
            drop(owned.take());
        }
        Poll::Ready(output)
    })
}
