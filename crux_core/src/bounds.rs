pub use thread_safety::{MaybeSend, MaybeSync};

#[cfg(feature = "unsync")]
mod thread_safety {
    pub trait MaybeSend {}
    pub trait MaybeSync {}

    impl<T> MaybeSend for T {}
    impl<T> MaybeSync for T {}
}

#[cfg(not(feature = "unsync"))]
mod thread_safety {
    pub trait MaybeSend: Send {}
    pub trait MaybeSync: Sync {}

    impl<T: Send> MaybeSend for T {}
    impl<T: Sync> MaybeSync for T {}
}
