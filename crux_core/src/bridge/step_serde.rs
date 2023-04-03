use crate::{capability::Operation, steps::Resolve, Step};

impl<Op> Step<Op>
where
    Op: Operation,
{
    pub fn serialize<'de, F, Eff>(self, effect: F) -> (Eff, Resolve<&'de [u8]>)
    where
        F: Fn(Op) -> Eff,
    {
        let Step(payload, resolve) = self;

        let resolve = resolve.map(|bytes| bcs::from_bytes(bytes).expect("Deserialization failed"));

        (effect(payload), resolve)
    }
}
