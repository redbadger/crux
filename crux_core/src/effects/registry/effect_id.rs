use std::marker::PhantomData;

// The id is a generational id, first 32 bits are the generation
// the second 32 are the index itself. That way we get to reuse
// the slots in the registry slab and still recognise a stale
// id if we are given one.

const INDEX_BITS: u32 = 32;
const INDEX_MASK: u64 = u32::MAX as u64;

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
    /// Reconstruct an `EffectId` from its raw integer representation.
    ///
    /// Use this on the resolve path to turn an id received back from a custom
    /// FFI (as a plain integer) into a typed `EffectId`. The `raw` value must be
    /// one previously produced by [`EffectId::into_raw`]; arbitrary integers may
    /// refer to no request, or to a stale slot.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self {
            raw,
            phantom: PhantomData,
        }
    }

    /// Unpack the id into its raw integer representation for passing across a
    /// custom FFI boundary.
    ///
    /// The returned value packs both the slab index and the generation, and can
    /// be turned back into an `EffectId` with [`EffectId::from_raw`].
    #[must_use]
    pub const fn into_raw(self) -> u64 {
        self.raw
    }

    pub(crate) fn new(index: usize, generation: u32) -> Self {
        let index = u32::try_from(index).expect("ParkedEffectId index overflow");
        Self::from_raw((u64::from(generation) << INDEX_BITS) | u64::from(index))
    }

    pub(crate) const fn index(self) -> usize {
        (self.raw & INDEX_MASK) as usize
    }

    pub(crate) const fn generation(self) -> u32 {
        (self.raw >> INDEX_BITS) as u32
    }
}
