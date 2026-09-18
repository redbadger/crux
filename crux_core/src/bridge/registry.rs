use std::collections::HashMap;
use std::sync::Mutex;

use facet::Facet;
use serde::{Deserialize, Serialize};

use super::{BridgeError, FfiFormat, Request};
use crate::bridge::request_serde::ResolveSerialized;
use crate::{EffectFFI, EffectVariant, OperationKind, ResolveError};

/// Identifies one request across the FFI boundary, for as long as anything
/// could still refer to it.
///
/// An id is structured rather than a bare counter: it names which effect the
/// request carries, how many times the shell is expected to resolve it, and an
/// ascending sequence number. A resolve that arrives for the wrong effect, the
/// wrong kind of request, or a request that has already finished can therefore
/// be reported as what it is, and a log line carrying an id says which effect
/// and which request it belonged to.
///
/// Sequence numbers ascend and are not reused when a request completes, so an
/// id that has been resolved stays unusable rather than being handed to some
/// unrelated later request.
///
/// Id `0` is reserved for notifications. Nothing is stored for a request the
/// core will never wait on, so every notification carries that same id, and
/// resolving one is a [`ResolveError::Never`] rather than a lookup miss.
///
/// The bit layout is the bridge's own business and may change. On the Rust
/// side, read the pieces through [`effect_index`](Self::effect_index),
/// [`kind`](Self::kind) and [`sequence`](Self::sequence) rather than by
/// picking the integer apart. Nothing is generated for the shell to read them
/// with: a shell resolves with the id exactly as it arrived, and how many
/// times to do so is answered by the `OperationKind` accessor on the effect,
/// not by anything in the id.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Facet, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[facet(transparent)]
pub struct EffectId(pub u32);

impl EffectId {
    /// The id every notification is issued with.
    pub const NOTIFICATION: Self = Self(0);

    /// The effect's variant index sits in the top 8 bits, above the kind bit
    /// and the sequence.
    const EFFECT_SHIFT: u32 = Sequence::BITS + 1;

    /// The one bit between the effect index and the sequence: set for a
    /// stream, clear for a request. (A notification is [`Self::NOTIFICATION`],
    /// so it needs no pattern of its own.)
    const STREAM_BIT: u32 = 1 << Sequence::BITS;

    /// Build an id for a request of `kind`, carrying the effect variant
    /// `effect_index`, numbered `sequence`.
    fn new(effect_index: u8, kind: OperationKind, sequence: Sequence) -> Self {
        match kind {
            OperationKind::Notify => Self::NOTIFICATION,
            OperationKind::Request => {
                Self((u32::from(effect_index) << Self::EFFECT_SHIFT) | sequence.0)
            }
            OperationKind::Stream => Self(
                (u32::from(effect_index) << Self::EFFECT_SHIFT) | Self::STREAM_BIT | sequence.0,
            ),
        }
    }

    /// Which variant of the effect enum this request carries, counting from
    /// zero in declaration order.
    ///
    /// Always `0` for a notification, and for any effect whose
    /// [`EffectFFI`] implementation is hand-written and does not override
    /// [`EffectFFI::variant_index`].
    #[must_use]
    // A shift of 24 leaves 8 bits, so nothing can be lost.
    #[allow(clippy::cast_possible_truncation)]
    pub const fn effect_index(self) -> u8 {
        (self.0 >> Self::EFFECT_SHIFT) as u8
    }

    /// How many times the shell is expected to resolve this request.
    ///
    /// This is the kind the request was registered with, which for an
    /// operation that declares [`Operation::KIND`](crate::capability::Operation::KIND)
    /// is the kind that operation always has, and otherwise the one the call
    /// site chose.
    #[must_use]
    pub const fn kind(self) -> OperationKind {
        if self.0 == Self::NOTIFICATION.0 {
            OperationKind::Notify
        } else if self.0 & Self::STREAM_BIT == 0 {
            OperationKind::Request
        } else {
            OperationKind::Stream
        }
    }

    /// The ascending number this id was issued with, for logging. Resolve with
    /// the whole [`EffectId`], not with this.
    ///
    /// `0` for a notification, which is the one id that is not sequenced.
    #[must_use]
    pub const fn sequence(self) -> u32 {
        self.0 & Sequence::MASK
    }
}

/// The low 23 bits of an [`EffectId`] — eight million ids — leaving the top
/// nine for the effect index and the kind bit.
///
/// A `Sequence` cannot hold a value that reaches those top bits, so advancing
/// one can never carry into them and change the effect or the kind. It is also
/// never zero, which is what keeps [`EffectId::NOTIFICATION`] out of the range
/// of ids a real request can be issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sequence(u32);

impl Sequence {
    const BITS: u32 = 23;
    const MASK: u32 = (1 << Self::BITS) - 1;

    /// The first sequence issued. Numbering starts at one so that no request
    /// can be given the notification id.
    pub const FIRST: Self = Self(1);
    #[cfg(test)]
    pub const LAST: Self = Self(Self::MASK);

    /// `None` if `sequence` is zero or would not fit below the kind bit.
    ///
    /// The registry only ever needs [`Sequence::FIRST`] and [`Sequence::next`],
    /// so this exists for tests: it is what makes an out-of-range sequence
    /// unrepresentable rather than merely unlikely.
    #[cfg(test)]
    pub const fn new(sequence: u32) -> Option<Self> {
        if sequence == 0 || sequence > Self::MASK {
            None
        } else {
            Some(Self(sequence))
        }
    }

    /// The next sequence, wrapping within its own bits — back to
    /// [`Sequence::FIRST`], never to zero.
    pub const fn next(self) -> Self {
        if self.0 == Self::MASK {
            Self::FIRST
        } else {
            Self(self.0 + 1)
        }
    }
}

pub struct ResolveRegistry<T: FfiFormat>(Mutex<Outstanding<T>>);

/// One request the shell could still resolve.
struct Entry<T: FfiFormat> {
    /// The id the request was issued with, kept so that a resolve carrying the
    /// right sequence but the wrong effect or kind can be told apart from one
    /// for a sequence nothing was ever issued under.
    id: EffectId,
    resolve: ResolveSerialized<T>,
}

/// The requests the shell could still resolve, keyed by the *sequence* of the
/// id they were given.
///
/// Keying by sequence rather than by the whole id is what lets a mangled id be
/// reported as mangled: an id whose sequence is outstanding but whose effect
/// index or kind bit disagrees with what was issued is a bug worth naming, not
/// a plain miss.
struct Outstanding<T: FfiFormat> {
    entries: HashMap<u32, Entry<T>>,
    next_sequence: Sequence,
    /// What the effect enum looks like, learned from the effects that have
    /// been registered. `None` until the first one, when there is nothing
    /// outstanding to resolve anyway.
    effect: Option<EffectInfo>,
}

/// What an error needs to know about the effect enum.
///
/// The registry is not generic over the effect type — it holds requests for
/// every variant at once — so what it needs from that type is copied out of it
/// when an effect is registered.
#[derive(Clone, Copy)]
struct EffectInfo {
    /// How many variants the effect enum has.
    count: u16,
    /// Names the variant at an index, where the effect implementation knows
    /// it — [`EffectFFI::variant_name`], which needs no effect to call, only
    /// the index an id carries.
    name: fn(u8) -> Option<&'static str>,
    /// The effect type's name, for an error about an index that names no
    /// variant and so has no name of its own to give.
    type_name: &'static str,
}

impl EffectInfo {
    fn of<Eff: EffectFFI>() -> Self {
        Self {
            count: Eff::VARIANT_COUNT,
            name: Eff::variant_name,
            type_name: std::any::type_name::<Eff>(),
        }
    }
}

impl<T: FfiFormat> Outstanding<T> {
    /// Issue the next id, for a request of `kind` carrying the effect variant
    /// `effect_index`.
    ///
    /// Every notification gets [`EffectId::NOTIFICATION`] and consumes no
    /// sequence: nothing is stored for one, so there is nothing to tell apart.
    ///
    /// Sequences ascend rather than filling gaps, so resolving a completed
    /// request is a lookup miss instead of a hit on whichever request happened
    /// to inherit its storage. The sequence wraps within its 23 bits; ids still
    /// outstanding are stepped over, so a live request can never be displaced
    /// even then.
    fn issue_id(&mut self, effect_index: u8, kind: OperationKind) -> EffectId {
        if matches!(kind, OperationKind::Notify) {
            return EffectId::NOTIFICATION;
        }

        loop {
            let sequence = self.next_sequence;
            self.next_sequence = sequence.next();

            if !self.entries.contains_key(&sequence.0) {
                return EffectId::new(effect_index, kind, sequence);
            }
        }
    }

    /// The entry `id` refers to, or why it refers to nothing.
    ///
    /// Everything here happens before the response bytes are looked at, so a
    /// bad id is never reported as bad output.
    fn entry(&mut self, id: EffectId) -> Result<&mut Entry<T>, ResolveError> {
        // Nothing is stored for a notification, and they all share one id, so
        // the id alone says what happened.
        if id.0 == EffectId::NOTIFICATION.0 {
            return Err(ResolveError::Never);
        }

        let effect = self.effect;

        if let Some(effect) = effect
            && u16::from(id.effect_index()) >= effect.count
        {
            return Err(ResolveError::NoSuchEffect {
                id: id.0,
                index: id.effect_index(),
                variants: effect.count,
                effect: effect.type_name,
            });
        }

        // Nothing has been registered yet if `effect` is `None`, so there is
        // no name to give; the index alone still says what the id claimed.
        let variant = |index| EffectVariant {
            index,
            name: effect.and_then(|effect| (effect.name)(index)),
        };

        let sequence = id.sequence();

        let Some(entry) = self.entries.get_mut(&sequence) else {
            return Err(ResolveError::NotFound(id.0.into()));
        };

        if entry.id.effect_index() != id.effect_index() {
            return Err(ResolveError::WrongEffect {
                id: id.0,
                sequence,
                expected: variant(entry.id.effect_index()),
                actual: variant(id.effect_index()),
            });
        }

        if entry.id.kind() != id.kind() {
            return Err(ResolveError::WrongKind {
                id: id.0,
                sequence,
                expected: entry.id.kind(),
                actual: id.kind(),
            });
        }

        Ok(entry)
    }
}

impl<T: FfiFormat> Default for ResolveRegistry<T> {
    fn default() -> Self {
        Self(Mutex::new(Outstanding {
            entries: HashMap::new(),
            next_sequence: Sequence::FIRST,
            effect: None,
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
        // `serialize` consumes the effect, so ask which variant it is first.
        let effect_index = effect.variant_index();
        let (effect, resolve) = effect.serialize();
        let kind = resolve.kind();

        let id = {
            let mut outstanding = self.0.lock().expect("Registry Mutex poisoned.");
            outstanding.effect = Some(EffectInfo::of::<Eff>());
            let id = outstanding.issue_id(effect_index, kind);

            // A request that cannot be resolved has nothing worth keeping: storing
            // one would add an entry per fire-and-forget effect — every render, for
            // the life of the process — that nothing would ever remove.
            if kind != OperationKind::Notify {
                outstanding
                    .entries
                    .insert(id.sequence(), Entry { id, resolve });
            }

            id
        };

        Request { id, effect }
    }
    // ANCHOR_END: register

    /// Resume a previously registered effect.
    ///
    /// Fails with [`ResolveError::Never`] if `id` is
    /// [`EffectId::NOTIFICATION`], which the core never waits on;
    /// [`ResolveError::NoSuchEffect`], [`ResolveError::WrongEffect`] or
    /// [`ResolveError::WrongKind`] if the id does not describe a request that
    /// could have been issued; and [`ResolveError::NotFound`] if it is simply
    /// not outstanding — never issued, or already resolved.
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

        let entry = outstanding.entry(id)?;

        let resolved = entry.resolve.resolve(response);

        // A `Once` turns itself into a `Never` as it resolves: the request is
        // finished, and its id will not be issued again, so drop it.
        let finished = matches!(entry.resolve, ResolveSerialized::Never);
        if finished {
            outstanding.entries.remove(&id.sequence());
        }

        resolved
    }
}

#[cfg(test)]
mod tests {
    use super::{EffectId, EffectInfo, Entry, Outstanding, ResolveSerialized, Sequence};
    use crate::bridge::JsonFfiFormat;
    use crate::{EffectVariant, OperationKind, ResolveError};
    use std::collections::HashMap;

    fn outstanding(next_sequence: Sequence) -> Outstanding<JsonFfiFormat> {
        Outstanding {
            entries: HashMap::new(),
            next_sequence,
            effect: None,
        }
    }

    /// A stand-in for the `#[effect]` macro's work: an effect enum of `count`
    /// variants, the first four of them named, so that the tests can see the
    /// names reach the errors.
    fn effect(count: u16) -> EffectInfo {
        fn name(index: u8) -> Option<&'static str> {
            ["Render", "Http", "KeyValue", "Timer"]
                .get(usize::from(index))
                .copied()
        }

        EffectInfo {
            count,
            name,
            type_name: "my_app::Effect",
        }
    }

    fn sequence(sequence: u32) -> Sequence {
        Sequence::new(sequence).expect("sequence should fit")
    }

    /// Park a `Never` under `id`, as though it had been issued and the shell
    /// still owed a response.
    fn park(outstanding: &mut Outstanding<JsonFfiFormat>, id: EffectId) {
        outstanding.entries.insert(
            id.sequence(),
            Entry {
                id,
                resolve: ResolveSerialized::Never,
            },
        );
    }

    // -----------------------------------------------------------------------
    // The layout
    // -----------------------------------------------------------------------

    #[test]
    fn an_id_carries_its_effect_kind_and_sequence() {
        let id = EffectId::new(7, OperationKind::Request, sequence(42));

        assert_eq!(id.effect_index(), 7);
        assert_eq!(id.kind(), OperationKind::Request);
        assert_eq!(id.sequence(), 42);
    }

    #[test]
    fn the_kind_bit_is_set_for_a_stream() {
        let request = EffectId::new(1, OperationKind::Request, sequence(1));
        let stream = EffectId::new(1, OperationKind::Stream, sequence(1));

        assert_eq!(stream.0 - request.0, 1 << 23, "stream is the bit above 23");
        assert_eq!(request.kind(), OperationKind::Request);
        assert_eq!(stream.kind(), OperationKind::Stream);
    }

    #[test]
    fn every_notification_gets_the_reserved_id() {
        for effect_index in [0, 1, 255] {
            let id = EffectId::new(effect_index, OperationKind::Notify, Sequence::LAST);

            assert_eq!(id, EffectId::NOTIFICATION);
            assert_eq!(id.0, 0);
            assert_eq!(id.kind(), OperationKind::Notify);
            assert_eq!(id.sequence(), 0);
        }
    }

    #[test]
    fn every_corner_of_the_layout_round_trips() {
        for effect_index in [0u8, 1, 128, 255] {
            for kind in [OperationKind::Request, OperationKind::Stream] {
                for seq in [1, 2, Sequence::MASK] {
                    let id = EffectId::new(effect_index, kind, sequence(seq));

                    assert_eq!(id.effect_index(), effect_index, "{id:?}");
                    assert_eq!(id.kind(), kind, "{id:?}");
                    assert_eq!(id.sequence(), seq, "{id:?}");
                    assert_ne!(id, EffectId::NOTIFICATION);
                }
            }
        }
    }

    #[test]
    fn a_sequence_cannot_be_built_out_of_range() {
        assert_eq!(Sequence::new(1), Some(Sequence::FIRST));
        assert_eq!(Sequence::new(Sequence::MASK), Some(Sequence::LAST));
        assert_eq!(Sequence::new(0), None, "zero is the notification's");
        assert_eq!(Sequence::new(Sequence::MASK + 1), None);
        assert_eq!(Sequence::new(u32::MAX), None);
    }

    #[test]
    fn the_last_sequence_wraps_to_the_first() {
        assert_eq!(Sequence::LAST.next(), Sequence::FIRST);
        assert_eq!(Sequence::FIRST.next(), sequence(2));
    }

    // -----------------------------------------------------------------------
    // Issuing ids
    // -----------------------------------------------------------------------

    #[test]
    fn sequences_start_at_one_and_are_never_reused() {
        let mut outstanding = outstanding(Sequence::FIRST);

        let ids: Vec<_> = (0..4)
            .map(|_| outstanding.issue_id(3, OperationKind::Request).sequence())
            .collect();
        assert_eq!(ids, vec![1, 2, 3, 4]);

        // Finishing a request frees its entry, but not its sequence.
        outstanding.entries.remove(&2);

        assert_eq!(
            outstanding.issue_id(3, OperationKind::Request).sequence(),
            5
        );
    }

    #[test]
    fn notifications_do_not_consume_a_sequence() {
        let mut outstanding = outstanding(Sequence::FIRST);

        assert_eq!(
            outstanding.issue_id(2, OperationKind::Notify),
            EffectId::NOTIFICATION
        );
        assert_eq!(
            outstanding.issue_id(2, OperationKind::Notify),
            EffectId::NOTIFICATION
        );

        assert_eq!(
            outstanding.issue_id(2, OperationKind::Request).sequence(),
            1
        );
    }

    #[test]
    fn wrapping_steps_over_outstanding_sequences() {
        let mut outstanding = outstanding(Sequence::LAST);

        // Still awaiting a response on 1 and 2 when the counter comes round.
        park(
            &mut outstanding,
            EffectId::new(0, OperationKind::Request, sequence(1)),
        );
        park(
            &mut outstanding,
            EffectId::new(0, OperationKind::Request, sequence(2)),
        );

        assert_eq!(
            outstanding.issue_id(0, OperationKind::Request).sequence(),
            Sequence::MASK
        );
        assert_eq!(
            outstanding.issue_id(0, OperationKind::Request).sequence(),
            3,
            "wrapping displaced a request that was still outstanding"
        );
    }

    #[test]
    fn wrapping_never_lands_on_the_notification_id() {
        let mut outstanding = outstanding(Sequence::LAST);

        let ids: Vec<_> = (0..3)
            .map(|_| outstanding.issue_id(0, OperationKind::Request))
            .collect();

        assert!(
            ids.iter().all(|id| *id != EffectId::NOTIFICATION),
            "an issued id collided with the notification id: {ids:?}"
        );
        assert_eq!(
            ids.iter().map(|id| id.sequence()).collect::<Vec<_>>(),
            vec![Sequence::MASK, 1, 2]
        );
    }

    // -----------------------------------------------------------------------
    // Rejecting ids the shell should never send back
    // -----------------------------------------------------------------------

    #[test]
    fn resolving_a_notification_says_so() {
        let mut outstanding = outstanding(Sequence::FIRST);

        let error = outstanding
            .entry(EffectId::NOTIFICATION)
            .err()
            .expect("resolving a notification should fail");

        assert!(matches!(error, ResolveError::Never), "{error}");
    }

    #[test]
    fn an_effect_index_outside_the_enum_is_rejected() {
        let mut outstanding = outstanding(Sequence::FIRST);
        outstanding.effect = Some(effect(3));
        park(
            &mut outstanding,
            EffectId::new(1, OperationKind::Request, sequence(1)),
        );

        let error = outstanding
            .entry(EffectId::new(9, OperationKind::Request, sequence(1)))
            .err()
            .expect("an unknown effect variant should be rejected");

        assert!(
            matches!(
                error,
                ResolveError::NoSuchEffect {
                    index: 9,
                    variants: 3,
                    effect: "my_app::Effect",
                    ..
                }
            ),
            "{error}"
        );
        assert_eq!(
            error.to_string(),
            "`my_app::Effect` has only 3 variants, but response id 0x09000001 carries variant 9."
        );
    }

    #[test]
    fn an_id_for_a_different_effect_is_rejected() {
        let mut outstanding = outstanding(Sequence::FIRST);
        outstanding.effect = Some(effect(4));
        park(
            &mut outstanding,
            EffectId::new(1, OperationKind::Request, sequence(1)),
        );

        let error = outstanding
            .entry(EffectId::new(2, OperationKind::Request, sequence(1)))
            .err()
            .expect("an id naming another effect should be rejected");

        assert!(
            matches!(
                error,
                ResolveError::WrongEffect {
                    sequence: 1,
                    expected: EffectVariant {
                        index: 1,
                        name: Some("Http")
                    },
                    actual: EffectVariant {
                        index: 2,
                        name: Some("KeyValue")
                    },
                    ..
                }
            ),
            "{error}"
        );
        assert_eq!(
            error.to_string(),
            "Request 1 expects `Http` (variant 1), but response id 0x02000001 carries `KeyValue` (variant 2)."
        );
    }

    /// An effect that does not name its variants — a hand-written
    /// [`EffectFFI`](crate::EffectFFI) — still gets an error that says which
    /// index the id claimed.
    #[test]
    fn an_unnamed_variant_is_reported_by_index_alone() {
        let mut outstanding = outstanding(Sequence::FIRST);
        outstanding.effect = Some(EffectInfo {
            count: 8,
            name: |_| None,
            type_name: "my_app::Effect",
        });
        park(
            &mut outstanding,
            EffectId::new(1, OperationKind::Request, sequence(1)),
        );

        let error = outstanding
            .entry(EffectId::new(2, OperationKind::Request, sequence(1)))
            .err()
            .expect("an id naming another effect should be rejected");

        assert_eq!(
            error.to_string(),
            "Request 1 expects variant 1, but response id 0x02000001 carries variant 2."
        );
    }

    /// Ids are rendered the same way by every error that reports a mangled
    /// one, so that one can be matched against another by eye.
    #[test]
    fn an_id_is_reported_in_hex() {
        let mut outstanding = outstanding(Sequence::FIRST);
        outstanding.effect = Some(effect(4));
        park(
            &mut outstanding,
            EffectId::new(1, OperationKind::Request, sequence(1)),
        );

        let error = outstanding
            .entry(EffectId::new(2, OperationKind::Request, sequence(1)))
            .err()
            .expect("an id naming another effect should be rejected");

        assert!(
            error.to_string().contains("response id 0x02000001 "),
            "{error}"
        );
    }

    #[test]
    fn an_id_with_the_wrong_kind_bit_is_rejected() {
        let mut outstanding = outstanding(Sequence::FIRST);
        outstanding.effect = Some(effect(4));
        park(
            &mut outstanding,
            EffectId::new(1, OperationKind::Request, sequence(1)),
        );

        let error = outstanding
            .entry(EffectId::new(1, OperationKind::Stream, sequence(1)))
            .err()
            .expect("an id claiming the wrong kind should be rejected");

        assert!(
            matches!(
                error,
                ResolveError::WrongKind {
                    sequence: 1,
                    expected: OperationKind::Request,
                    actual: OperationKind::Stream,
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn an_id_that_was_never_issued_is_still_not_found() {
        let mut outstanding = outstanding(Sequence::FIRST);
        outstanding.effect = Some(effect(4));

        let id = EffectId::new(1, OperationKind::Request, sequence(7));
        let error = outstanding
            .entry(id)
            .err()
            .expect("an unissued id should be rejected");

        assert!(
            matches!(error, ResolveError::NotFound(found) if found == u64::from(id.0)),
            "{error}"
        );
    }

    #[test]
    fn a_well_formed_id_finds_its_entry() {
        let mut outstanding = outstanding(Sequence::FIRST);
        outstanding.effect = Some(effect(4));
        let id = EffectId::new(2, OperationKind::Stream, sequence(1));
        park(&mut outstanding, id);

        assert!(outstanding.entry(id).is_ok());
    }
}
