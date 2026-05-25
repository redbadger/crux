use std::marker::PhantomData;

// The id is a generational id, first 32 bits are the generation
// the second 32 are the index itself. That way we get to reuse
// the slots in the registry slab and still recognise a stale
// id if we are given one.

pub(crate) const INDEX_BITS: u32 = 32;
pub(crate) const INDEX_MASK: u64 = u32::MAX as u64;

/// Opaque ID for a parked effect request.
///
/// The raw value packs a slab index and generation into a single integer so
/// callers can pass it over custom FFI boundaries without learning the storage
/// layout.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct EffectId<T = ()> {
    pub(crate) raw: u64,
    pub(crate) phantom: PhantomData<fn(T) -> T>,
}

impl<T> Clone for EffectId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for EffectId<T> {}

impl<T> EffectId<T> {
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self {
            raw,
            phantom: PhantomData,
        }
    }

    #[must_use]
    pub const fn into_raw(self) -> u64 {
        self.raw
    }

    pub(crate) fn new(index: usize, generation: u32) -> Self {
        let index = u32::try_from(index).expect("ParkedEffectId index overflow");
        Self::from_raw((u64::from(generation) << INDEX_BITS) | u64::from(index))
    }

    pub(crate) fn index(self) -> usize {
        (self.raw & INDEX_MASK) as usize
    }

    pub(crate) fn generation(self) -> u32 {
        (self.raw >> INDEX_BITS) as u32
    }
}
