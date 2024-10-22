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

            pub struct CapabilityOne {
                context: CapabilityContext<OpOne>,
            }

            // Needed to allow 'this = (*self).clone()' without requiring E: Clone
            // See https://github.com/rust-lang/rust/issues/26925
            impl Clone for CapabilityOne {
                fn clone(&self) -> Self {
                    Self {
                        context: self.context.clone(),
                    }
                }
            }

            impl CapabilityOne {
                #[must_use]
                pub fn new(context: CapabilityContext<OpOne>) -> Self {
                    Self { context }
                }

                pub async fn one_async(&self, number: usize) -> usize {
                    self.context.request_from_shell(OpOne { number }).await
                }
            }

            impl crux_core::Capability for CapabilityOne {
                type Operation = OpOne;
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

            pub struct CapabilityTwo {
                context: CapabilityContext<OpTwo>,
            }

            // Needed to allow 'this = (*self).clone()' without requiring E: Clone
            // See https://github.com/rust-lang/rust/issues/26925
            impl Clone for CapabilityTwo {
                fn clone(&self) -> Self {
                    Self {
                        context: self.context.clone(),
                    }
                }
            }

            impl CapabilityTwo {
                #[must_use]
                pub fn new(context: CapabilityContext<OpTwo>) -> Self {
                    Self { context }
                }

                pub async fn two_async(&self, number: usize) -> usize {
                    self.context.request_from_shell(OpTwo { number }).await
                }
            }

            impl crux_core::Capability for CapabilityTwo {
                type Operation = OpTwo;
            }
        }
    }
}
