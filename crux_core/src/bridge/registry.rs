use std::collections::HashMap;
use std::sync::Mutex;

use facet::Facet;
use serde::{Deserialize, Serialize};

use super::{BridgeError, FfiFormat, Request};
use crate::bridge::request_serde::ResolveSerialized;
use crate::{EffectFFI, RequestKind, ResolveError};

/// Identifies one request across the FFI boundary, for as long as anything
/// could still refer to it.
///
/// Ids are issued in ascending order and are not reused when a request
/// completes, so an id that has been resolved stays unusable rather than being
/// handed to some unrelated later request.
///
/// The top 2 bits hold the request's [`RequestKind`] and the low 30 bits its
/// [`Sequence`]. Read them through [`EffectId::kind`] and
/// [`EffectId::sequence`]; the encoding is an implementation detail.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Facet, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[facet(transparent)]
pub struct EffectId(pub u32);

impl EffectId {
    /// The kind sits above the 30 bits of sequence.
    const KIND_SHIFT: u32 = 30;
    const SEQUENCE_MASK: u32 = (1 << Self::KIND_SHIFT) - 1;

    const NOTIFY: u32 = 0;
    const REQUEST: u32 = 1;
    const STREAM: u32 = 2;
    // 3 is reserved.

    /// Build an id for a request of `kind`, numbered `sequence`.
    pub(crate) const fn new(kind: RequestKind, sequence: Sequence) -> Self {
        let tag = match kind {
            RequestKind::Notify => Self::NOTIFY,
            RequestKind::Request => Self::REQUEST,
            RequestKind::Stream => Self::STREAM,
        };

        Self((tag << Self::KIND_SHIFT) | sequence.0)
    }

    /// How many times this request expects to be resolved, or `None` if the
    /// id's kind bits hold the unused fourth pattern.
    #[must_use]
    pub const fn kind(self) -> Option<RequestKind> {
        match self.0 >> Self::KIND_SHIFT {
            Self::NOTIFY => Some(RequestKind::Notify),
            Self::REQUEST => Some(RequestKind::Request),
            Self::STREAM => Some(RequestKind::Stream),
            _ => None,
        }
    }

    /// The ascending number this id was issued with, for logging. Resolve with
    /// the whole [`EffectId`], not with this.
    #[must_use]
    pub const fn sequence(self) -> u32 {
        self.0 & Self::SEQUENCE_MASK
    }
}

/// The low 30 bits of an [`EffectId`] — a billion ids — leaving the top 2 for
/// the kind.
///
/// A `Sequence` cannot hold a value that reaches those top 2 bits, so
/// advancing one can never carry into the kind and change it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sequence(u32);

impl Sequence {
    pub const ZERO: Self = Self(0);
    #[cfg(test)]
    pub const LAST: Self = Self(EffectId::SEQUENCE_MASK);

    /// `None` if `sequence` would not fit below the kind.
    ///
    /// The registry only ever needs [`Sequence::ZERO`] and [`Sequence::next`],
    /// so this exists for tests: it is what makes an out-of-range sequence
    /// unrepresentable rather than merely unlikely.
    #[cfg(test)]
    pub const fn new(sequence: u32) -> Option<Self> {
        if sequence > EffectId::SEQUENCE_MASK {
            None
        } else {
            Some(Self(sequence))
        }
    }

    /// The next sequence, wrapping within its own bits.
    pub const fn next(self) -> Self {
        if self.0 == EffectId::SEQUENCE_MASK {
            Self::ZERO
        } else {
            Self(self.0 + 1)
        }
    }
}

pub struct ResolveRegistry<T: FfiFormat>(Mutex<Outstanding<T>>);

/// The requests the shell could still resolve, keyed by the id it was given.
struct Outstanding<T: FfiFormat> {
    entries: HashMap<u32, ResolveSerialized<T>>,
    /// One sequence for all three kinds; the kind bits keep their ids apart.
    next_sequence: Sequence,
}

impl<T: FfiFormat> Outstanding<T> {
    /// Issue the next id, for a request of `kind`.
    ///
    /// Sequences ascend rather than filling gaps, so resolving a completed
    /// request is a lookup miss rather than a hit on some unrelated request.
    /// The sequence wraps, stepping over ids still outstanding.
    fn issue_id(&mut self, kind: RequestKind) -> EffectId {
        loop {
            let id = EffectId::new(kind, self.next_sequence);
            self.next_sequence = self.next_sequence.next();

            if !self.entries.contains_key(&id.0) {
                return id;
            }
        }
    }
}

impl<T: FfiFormat> Default for ResolveRegistry<T> {
    fn default() -> Self {
        Self(Mutex::new(Outstanding {
            entries: HashMap::new(),
            next_sequence: Sequence::ZERO,
        }))
    }
}

impl<T: FfiFormat> ResolveRegistry<T> {
    /// Register an effect for future continuation, when it has been processed
    /// and output given back to the core.
    ///
    /// The `effect` will be serialized into its FFI counterpart before being stored
    /// and wrapped in a [`Request`].
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex has been poisoned
    // ANCHOR: register
    pub fn register<Eff>(&self, effect: Eff) -> Request<Eff::Ffi>
    where
        Eff: EffectFFI,
    {
        let (effect, resolve) = effect.serialize();
        let kind = resolve.kind();

        let id = {
            let mut outstanding = self.0.lock().expect("Registry Mutex poisoned.");
            let id = outstanding.issue_id(kind);

            // A request that cannot be resolved has nothing worth keeping: storing
            // one would add an entry per fire-and-forget effect — every render, for
            // the life of the process — that nothing would ever remove.
            if kind != RequestKind::Notify {
                outstanding.entries.insert(id.0, resolve);
            }

            id
        };

        Request { id, effect }
    }
    // ANCHOR_END: register

    /// Resume a previously registered effect.
    ///
    /// Fails with [`ResolveError::Never`] if `id` belongs to a fire-and-forget
    /// request, and [`ResolveError::NotFound`] if it is not outstanding for any
    /// other reason — never issued, or already resolved.
    ///
    /// # Errors
    ///
    /// Returns `BridgeError` if the stored request could not be resolved.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex has been poisoned
    pub fn resume(&self, id: EffectId, response: &[u8]) -> Result<(), BridgeError<T>> {
        let mut outstanding = self.0.lock().expect("Registry Mutex poisoned");

        let Some(entry) = outstanding.entries.get_mut(&id.0) else {
            // Nothing is stored for a fire-and-forget request, but its id still
            // says what it was.
            return Err(BridgeError::ProcessResponse(
                if id.kind() == Some(RequestKind::Notify) {
                    ResolveError::Never
                } else {
                    ResolveError::NotFound(id.0.into())
                },
            ));
        };

        let resolved = entry.resolve(response);

        // A `Once` turns itself into a `Never` as it resolves: the request is
        // finished, and its id will not be issued again, so drop it.
        if matches!(entry, ResolveSerialized::Never) {
            outstanding.entries.remove(&id.0);
        }

        resolved
    }
}

#[cfg(test)]
mod tests {
    use super::{EffectId, Outstanding, ResolveSerialized, Sequence};
    use crate::RequestKind;
    use crate::bridge::JsonFfiFormat;
    use std::collections::HashMap;

    const KINDS: [RequestKind; 3] = [
        RequestKind::Notify,
        RequestKind::Request,
        RequestKind::Stream,
    ];

    fn outstanding(next_sequence: Sequence) -> Outstanding<JsonFfiFormat> {
        Outstanding {
            entries: HashMap::new(),
            next_sequence,
        }
    }

    fn sequence(sequence: u32) -> Sequence {
        Sequence::new(sequence).expect("sequence should fit")
    }

    #[test]
    fn a_sequence_cannot_be_built_out_of_range() {
        assert_eq!(Sequence::new(EffectId::SEQUENCE_MASK), Some(Sequence::LAST));
        assert_eq!(Sequence::new(EffectId::SEQUENCE_MASK + 1), None);
        assert_eq!(Sequence::new(u32::MAX), None);
    }

    #[test]
    fn the_last_sequence_wraps_to_zero() {
        assert_eq!(Sequence::LAST.next(), Sequence::ZERO);
        assert_eq!(Sequence::ZERO.next(), sequence(1));
    }

    #[test]
    fn an_id_carries_its_kind_and_its_sequence() {
        for kind in KINDS {
            for raw in [0, 1, 12345, EffectId::SEQUENCE_MASK] {
                let id = EffectId::new(kind, sequence(raw));

                assert_eq!(id.kind(), Some(kind));
                assert_eq!(id.sequence(), raw);
            }
        }
    }

    /// The increment at the top of the range is where a carry would reach the
    /// kind bits, turning a notification into a request or a request into a
    /// stream. Issue either side of the wrap and the kind must not move.
    #[test]
    fn wrapping_the_sequence_does_not_disturb_the_kind() {
        for kind in KINDS {
            let mut outstanding = outstanding(Sequence::LAST);

            let last = outstanding.issue_id(kind);
            assert_eq!(last.kind(), Some(kind));
            assert_eq!(last.sequence(), EffectId::SEQUENCE_MASK);

            let wrapped = outstanding.issue_id(kind);
            assert_eq!(wrapped.kind(), Some(kind), "the carry reached the kind");
            assert_eq!(wrapped.sequence(), 0);
        }
    }

    #[test]
    fn the_same_sequence_in_two_kinds_is_two_ids() {
        assert_ne!(
            EffectId::new(RequestKind::Request, sequence(7)),
            EffectId::new(RequestKind::Stream, sequence(7))
        );
    }

    #[test]
    fn an_id_with_no_known_kind_has_no_kind() {
        assert_eq!(EffectId(0b11 << EffectId::KIND_SHIFT).kind(), None);
    }

    #[test]
    fn ids_ascend_and_are_never_reused() {
        let mut outstanding = outstanding(Sequence::ZERO);

        let ids: Vec<_> = (0..4)
            .map(|_| outstanding.issue_id(RequestKind::Request))
            .collect();
        assert_eq!(
            ids.iter()
                .copied()
                .map(EffectId::sequence)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );

        // Finishing a request frees its entry, but not its id.
        outstanding.entries.remove(&ids[1].0);

        assert_eq!(outstanding.issue_id(RequestKind::Request).sequence(), 4);
    }

    #[test]
    fn wrapping_steps_over_outstanding_ids() {
        let mut outstanding = outstanding(Sequence::LAST);

        // Still awaiting a response on sequences 0 and 1 when the sequence wraps.
        for raw in [0, 1] {
            outstanding.entries.insert(
                EffectId::new(RequestKind::Request, sequence(raw)).0,
                ResolveSerialized::Never,
            );
        }

        assert_eq!(
            outstanding.issue_id(RequestKind::Request).sequence(),
            EffectId::SEQUENCE_MASK
        );
        assert_eq!(
            outstanding.issue_id(RequestKind::Request).sequence(),
            2,
            "wrapping displaced a request that was still outstanding"
        );
    }

    #[test]
    fn wrapping_only_steps_over_the_same_kind() {
        let mut outstanding = outstanding(Sequence::LAST);

        // A live stream at sequence 0 is a different id to a request at 0.
        outstanding.entries.insert(
            EffectId::new(RequestKind::Stream, Sequence::ZERO).0,
            ResolveSerialized::Never,
        );

        assert_eq!(
            outstanding.issue_id(RequestKind::Request).sequence(),
            EffectId::SEQUENCE_MASK
        );
        assert_eq!(outstanding.issue_id(RequestKind::Request).sequence(), 0);
    }
}
