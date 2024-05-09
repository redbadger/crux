//! This is support code for doc tests
pub mod compose {
    pub mod capabilities {
        pub mod capability_one {
            use crux_core::capability::{CapabilityContext, Operation};
            use serde::{Deserialize, Serialize};

            #[derive(PartialEq, Serialize, Deserialize, Debug)]
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
                    F: Fn(NewEv) -> Ev + Send + Sync + Copy + 'static,
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

            #[derive(PartialEq, Serialize, Deserialize, Debug)]
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
                    F: Fn(NewEv) -> Ev + Send + Sync + Copy + 'static,
                    Ev: 'static,
                    NewEv: 'static,
                {
                    CapabilityTwo::new(self.context.map_event(f))
                }
            }
        }
    }
}
