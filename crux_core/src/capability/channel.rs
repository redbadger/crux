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

    /// Receives a message if any are waiting.
    /// Returns the error branch if the sender has disconnected.
    ///
    /// This API isn't that nice, but isn't intended for public consumption
    /// so whatevs.
    pub fn try_receive(&self) -> Result<Option<T>, ()> {
        match self.inner.try_recv() {
            Ok(inner) => Ok(Some(inner)),
            Err(crossbeam_channel::TryRecvError::Empty) => Ok(None),
            Err(crossbeam_channel::TryRecvError::Disconnected) => Err(()),
        }
    }

    pub fn drain(&self) -> Drain<T> {
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
        self.inner.send(t)
    }

    pub fn map_input<NewT, F>(&self, func: F) -> Sender<NewT>
    where
        F: Fn(NewT) -> T + Send + Sync + 'static,
    {
        Sender {
            inner: Arc::new(MappedInner {
                sender: Arc::clone(&self.inner),
                func,
            }),
        }
    }
}

trait SenderInner<T> {
    fn send(&self, t: T);
}

impl<T> SenderInner<T> for crossbeam_channel::Sender<T> {
    fn send(&self, t: T) {
        crossbeam_channel::Sender::send(self, t).unwrap()
    }
}

pub struct MappedInner<T, F> {
    sender: Arc<dyn SenderInner<T> + Send + Sync>,
    func: F,
}

impl<F, T, U> SenderInner<U> for MappedInner<T, F>
where
    F: Fn(U) -> T,
{
    fn send(&self, value: U) {
        self.sender.send((self.func)(value))
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

        let wrapped_send = send.map_input(Some);
        wrapped_send.send(1);
        assert_eq!(recv.receive(), Some(Some(1)));

        assert_eq!(recv.receive(), None);
    }
}
