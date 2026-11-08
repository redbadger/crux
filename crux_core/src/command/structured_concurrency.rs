//! Completion and cancellation links between independently hosted commands.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use super::{AbortHandle, CommandContext, JoinHandle};

/// Keeps an emitting command waiting for its root-hosted follow-up.
///
/// Polling the tracker only polls the join handle, never the child command.
/// Dropping it before completion requests cancellation of that child, including
/// when the parent is aborted before the tracker's first poll.
pub struct ChildTracker {
    /// Completion of the child's hosting task on the root command.
    join_handle: JoinHandle,
    /// Cancels the child command when the tracker is dropped early.
    abort_handle: AbortHandle,
    /// Prevents normal completion from requesting an unnecessary abort.
    finished: bool,
}

impl ChildTracker {
    pub(crate) const fn new(join_handle: JoinHandle, abort_handle: AbortHandle) -> Self {
        Self {
            join_handle,
            abort_handle,
            finished: false,
        }
    }
}

impl Future for ChildTracker {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        std::task::ready!(Pin::new(&mut self.join_handle).poll(cx));
        self.finished = true;
        Poll::Ready(())
    }
}

impl Drop for ChildTracker {
    fn drop(&mut self) {
        if !self.finished {
            self.abort_handle.abort();
        }
    }
}

/// Keeps the parent unfinished until the child has been cleaned up.
///
/// Cancellation drops the child tracker before the root cleans up the child.
/// This guard retains the parent's context in the meantime, then releases it
/// and wakes the parent on drop.
pub struct ParentCompletionGuard<Effect, Event> {
    /// Taken during Drop so the reference is released before waking the parent.
    context: Option<CommandContext<Effect, Event>>,
}

impl<Effect, Event> ParentCompletionGuard<Effect, Event> {
    /// Construct before entering the child's async block so an unpolled future
    /// also releases and wakes its parent when dropped.
    pub(crate) const fn new(context: CommandContext<Effect, Event>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

impl<Effect, Event> Drop for ParentCompletionGuard<Effect, Event> {
    fn drop(&mut self) {
        if let Some(context) = self.context.take() {
            let waker = context.host_waker.clone();
            drop(context);
            waker.wake();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use futures::channel::oneshot;

    use super::{ChildTracker, ParentCompletionGuard};
    use crate::Command;

    struct Cleanup(Arc<AtomicBool>);

    impl Drop for Cleanup {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Relaxed);
        }
    }

    #[test]
    fn aborting_parent_before_tracker_is_polled_waits_for_child_cleanup() {
        let cleaned_up = Arc::new(AtomicBool::new(false));
        let cleanup = Cleanup(cleaned_up.clone());
        let (_sender, receiver) = oneshot::channel::<()>();
        let child: Command<(), ()> = Command::new(move |_| async move {
            let _cleanup = cleanup;
            let _ = receiver.await;
        });
        let abort_handle = child.abort_handle();
        let mut parent: Command<(), ()> = Command::done();
        let mut root: Command<(), ()> = Command::done();
        let parent_guard = ParentCompletionGuard::new(parent.context.clone());
        let join_handle = root.spawn(|ctx| async move {
            child.into_future(ctx).await;
            drop(parent_guard);
        });
        parent.spawn(move |_| ChildTracker::new(join_handle, abort_handle));

        parent.abort_handle().abort();
        assert!(!parent.is_done(), "parent must wait for child cleanup");
        assert!(!cleaned_up.load(Ordering::Relaxed));
        assert!(root.is_done(), "root must process the child's cancellation");
        assert!(cleaned_up.load(Ordering::Relaxed));
        assert!(parent.is_done());
    }

    #[test]
    fn child_completion_releases_parent_without_requesting_an_abort() {
        let child: Command<(), ()> = Command::done();
        let abort_handle = child.abort_handle();
        let mut parent: Command<(), ()> = Command::done();
        let mut root: Command<(), ()> = Command::done();
        let parent_guard = ParentCompletionGuard::new(parent.context.clone());
        let join_handle = root.spawn(|ctx| async move {
            child.into_future(ctx).await;
            drop(parent_guard);
        });
        parent.spawn(|_| ChildTracker::new(join_handle, abort_handle.clone()));

        assert!(!parent.is_done());
        assert!(root.is_done());
        assert!(parent.is_done());
        assert!(!abort_handle.aborted.load(Ordering::Relaxed));
    }
}
