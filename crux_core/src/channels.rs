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
    pub fn receive(&self) -> Option<T> {
        match self.inner.try_recv() {
            Ok(inner) => Some(inner),
            Err(crossbeam_channel::TryRecvError::Empty) => None,
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                panic!("Receiver was disconnected.")
                // TODO: Should this be a panic or just return None?  Not sure
            }
        }
    }
}

pub struct Sender<T> {
    inner: Arc<dyn SenderInner<T> + Send + Sync>,
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

impl<Ef, Ev> Sender<crate::Command<Ef, Ev>>
where
    Ev: 'static,
    Ef: 'static,
{
    pub fn map_effect<NewEf, F>(&self, func: F) -> Sender<crate::Command<NewEf, Ev>>
    where
        F: Fn(NewEf) -> Ef + Sync + Send + Copy + 'static,
        NewEf: 'static,
    {
        self.map_input::<crate::Command<_, _>, _>(move |command| command.map_effect(func))
    }

    pub fn map_event<NewEv, F>(&self, func: F) -> Sender<crate::Command<Ef, NewEv>>
    where
        F: Fn(NewEv) -> Ev + Sync + Send + Copy + 'static,
        NewEv: 'static,
    {
        self.map_input::<crate::Command<_, _>, _>(move |command| command.map(func))
    }
}

trait SenderInner<T> {
    fn send(&self, t: T);
    // fn map_input(&self, func: F) -> Sender<F>;
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

    // fn map_input(&self, func: F) -> Sender<F> {
    //     Sender {
    //         inner: MappedInner {
    //             sender: self.sender.clone(),
    //             func,
    //         },
    //     }
    // }
}

// TOOD: Some tests that this meets safe/sync requirements, is compatible with wasm etc.

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
