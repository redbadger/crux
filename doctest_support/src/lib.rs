//! This is support code for doc tests

pub mod command {
    use crux_core::{capability::Operation, Request};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Clone, Serialize)]
    pub enum AnOperation {
        One(u8),
        Two(u8),
    }

    #[derive(Debug, PartialEq, Deserialize)]
    pub enum AnOperationOutput {
        One(u8),
        Two(u8),
    }

    impl Operation for AnOperation {
        type Output = AnOperationOutput;
    }

    pub enum Effect {
        AnEffect(Request<AnOperation>),
        Http(Request<crux_http::protocol::HttpRequest>),
        Render(Request<crux_core::render::RenderOperation>),
    }

    impl From<Request<AnOperation>> for Effect {
        fn from(request: Request<AnOperation>) -> Self {
            Self::AnEffect(request)
        }
    }

    impl From<Request<crux_http::protocol::HttpRequest>> for Effect {
        fn from(request: Request<crux_http::protocol::HttpRequest>) -> Self {
            Self::Http(request)
        }
    }

    impl From<Request<crux_core::render::RenderOperation>> for Effect {
        fn from(request: Request<crux_core::render::RenderOperation>) -> Self {
            Self::Render(request)
        }
    }

    #[derive(Debug, PartialEq, Deserialize)]
    pub struct Post {
        pub url: String,
        pub title: String,
        pub body: String,
    }

    #[derive(Debug, PartialEq)]
    pub enum Event {
        Start,
        Completed(AnOperationOutput),
        Aborted,
        GotPost(Result<crux_http::Response<Post>, crux_http::HttpError>),
    }
}

pub mod compose {
    pub mod capabilities {
        pub mod capability_one {
            use crux_core::capability::{CapabilityContext, Operation};
            use serde::{Deserialize, Serialize};

            #[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
            pub struct OpOne {
                number: usize,
            }

            impl Operation for OpOne {
                type Output = usize;
            }

            pub struct CapabilityOne<E> {
                context: CapabilityContext<OpOne, E>,
            }

            // Needed to allow 'this = (*self).clone()' without requiring E: Clone
            // See https://github.com/rust-lang/rust/issues/26925
            impl<E> Clone for CapabilityOne<E> {
                fn clone(&self) -> Self {
                    Self {
                        context: self.context.clone(),
                    }
                }
            }

            impl<E> CapabilityOne<E> {
                #[must_use]
                pub fn new(context: CapabilityContext<OpOne, E>) -> Self {
                    Self { context }
                }

                pub fn one<F>(&self, number: usize, event: F)
                where
                    F: FnOnce(usize) -> E + Send + 'static,
                    E: 'static,
                {
                    let this = Clone::clone(self);

                    this.context.spawn({
                        let this = this.clone();

                        async move {
                            let result = this.one_async(number).await;

                            this.context.update_app(event(result));
                        }
                    });
                }

                pub async fn one_async(&self, number: usize) -> usize
                where
                    E: 'static,
                {
                    self.context.request_from_shell(OpOne { number }).await
                }
            }

            impl<Ev> crux_core::Capability<Ev> for CapabilityOne<Ev> {
                type Operation = OpOne;
                type MappedSelf<MappedEv> = CapabilityOne<MappedEv>;

                fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
                where
                    F: Fn(NewEv) -> Ev + Send + Sync + 'static,
                    Ev: 'static,
                    NewEv: 'static,
                {
                    CapabilityOne::new(self.context.map_event(f))
                }
            }
        }

        pub mod capability_two {
            use crux_core::capability::{CapabilityContext, Operation};
            use serde::{Deserialize, Serialize};

            #[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
            pub struct OpTwo {
                number: usize,
            }

            impl Operation for OpTwo {
                type Output = usize;
            }

            pub struct CapabilityTwo<E> {
                context: CapabilityContext<OpTwo, E>,
            }

            // Needed to allow 'this = (*self).clone()' without requiring E: Clone
            // See https://github.com/rust-lang/rust/issues/26925
            impl<E> Clone for CapabilityTwo<E> {
                fn clone(&self) -> Self {
                    Self {
                        context: self.context.clone(),
                    }
                }
            }

            impl<E> CapabilityTwo<E> {
                #[must_use]
                pub fn new(context: CapabilityContext<OpTwo, E>) -> Self {
                    Self { context }
                }

                pub fn two<F>(&self, number: usize, event: F)
                where
                    F: FnOnce(usize) -> E + Send + 'static,
                    E: 'static,
                {
                    let this = Clone::clone(self);

                    this.context.spawn({
                        let this = this.clone();

                        async move {
                            let result = this.two_async(number).await;

                            this.context.update_app(event(result));
                        }
                    });
                }

                pub async fn two_async(&self, number: usize) -> usize
                where
                    E: 'static,
                {
                    self.context.request_from_shell(OpTwo { number }).await
                }
            }

            impl<Ev> crux_core::Capability<Ev> for CapabilityTwo<Ev> {
                type Operation = OpTwo;
                type MappedSelf<MappedEv> = CapabilityTwo<MappedEv>;

                fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
                where
                    F: Fn(NewEv) -> Ev + Send + Sync + 'static,
                    Ev: 'static,
                    NewEv: 'static,
                {
                    CapabilityTwo::new(self.context.map_event(f))
                }
            }
        }
    }
}
