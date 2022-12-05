/// TODO docs!
pub trait Capability {}

pub trait Capabilities<C> {
    fn get(&self) -> &C;
}
