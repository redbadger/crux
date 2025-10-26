// Wrappers around crossbeam_channel that only expose the functionality we need (and is safe on wasm)

use std::sync::Arc;

pub(crate) fn channel<T>() -> (Sender<T>, Receiver<T>)
where
    T: Send + 'static,
{
    let (sender, receiver) = crossbeam_channel::unbounded();
    let sender = Sender {
        inner: Arc::new(sender),
    };
    let receiver = Receiver { inner: receiver };

    (sender, receiver)
}

pub struct Receiver<T> {
    inner: crossbeam_channel::Receiver<T>,
}

impl<T> Receiver<T> {
    /// Receives a message if any are waiting.
    ///
    /// Panics if the receiver has disconnected, so shouldn't be used if
    /// that's possible.
    pub fn receive(&self) -> Option<T> {
        match self.inner.try_recv() {
            Ok(inner) => Some(inner),
            Err(crossbeam_channel::TryRecvError::Empty) => None,
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                // Users _generally_ shouldn't be messing with channels themselves, so
                // this probably shouldn't happen.  Might happen in tests, but lets
                // fix that if we get complaints
                panic!("Receiver was disconnected.")
            }
        }
    }

    pub fn drain(&self) -> Drain<'_, T> {
        Drain { receiver: self }
    }
}

pub struct Drain<'a, T> {
    receiver: &'a Receiver<T>,
}

impl<T> Iterator for Drain<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.receive()
    }
}

pub struct Sender<T> {
    inner: Arc<dyn SenderInner<T> + Send + Sync>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Sender<T>
where
    T: 'static,
{
    pub fn send(&self, t: T) {
        self.inner.send(t);
    }
}

trait SenderInner<T> {
    fn send(&self, t: T);
}

impl<T> SenderInner<T> for crossbeam_channel::Sender<T> {
    fn send(&self, t: T) {
        crossbeam_channel::Sender::send(self, t).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use static_assertions::assert_impl_all;

    use super::*;

    assert_impl_all!(Sender<i32>: Send);

    #[test]
    fn test_channels() {
        let (send, recv) = channel();

        send.send(Some(1));
        assert_eq!(recv.receive(), Some(Some(1)));

        assert_eq!(recv.receive(), None);
    }
}
