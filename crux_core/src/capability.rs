/// TODO docs!
pub trait Capability {}

pub trait GetCapabilityInstance {
    type Capability;

    fn capability(&self) -> Self::Capability;
}
