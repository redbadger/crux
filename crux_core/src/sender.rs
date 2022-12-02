use std::sync::mpsc::{SendError, Sender};

pub trait CruxSender<T> {
    // type Err;

    fn send(&self, t: T);
}

pub trait SenderExt<T>: CruxSender<T> {
    fn map_input<NewT, F>(self, func: F) -> Map<Self, F>
    where
        Self: Sized,
        F: Fn(NewT) -> T,
    {
        Map { sender: self, func }
    }
}

impl<T> CruxSender<T> for Sender<T> {
    // type Err = T;

    fn send(&self, value: T) {
        self.send(value).expect("TOOD: do something about this ")
    }
}

impl<T> SenderExt<T> for Sender<T> {}

pub struct Map<S, F> {
    sender: S,
    func: F,
}

impl<S: CruxSender<U>, F, T, U> CruxSender<T> for Map<S, F>
where
    F: Fn(T) -> U,
{
    // type Err = S::Err;

    fn send(&self, value: T) {
        self.sender.send((self.func)(value))
    }
}
