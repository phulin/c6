//! Concrete Linux current-handler lookup and interruption leaves.
//!
//! Admission must retain the actual Manager/handler owners, verify its delegated
//! virtual slots, and supply observations from the matching module/configuration
//! and synchronous context. Saved queue hashes are not live queue front IDs.
//! Current closed captures contain vptrs but no live +0x20 slot observations.
//! Image relocations cannot admit this provider. This is source closure, not a
//! native Manager predicate comparison.
use crate::messages::{HandlerId, OperationHash};

const GAME_SHA: &str = "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa";

/// Evidence identity supplied by the admitting owner. Configuration and capture
/// hashes identify the observations; this library cannot authenticate live owners.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationIdentity<'a> {
    pub gamecore_sha256: &'a str,
    pub configuration_sha256: &'a str,
    pub capture_sha256: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservedHandler {
    pub runtime_id: HandlerId,
    pub operation_hash: OperationHash,
    /// Actual runtime slot observation is required; an image-relocation prediction
    /// is explicitly rejected, even when it names a supported original leaf.
    pub interruption_target: InterruptionTarget,
}

/// Provenance must be supplied by the capture owner. Public values cannot
/// authenticate native memory: a caller must never label static ELF predictions
/// or synthetic test inputs as a genuine live observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterruptionTarget {
    /// Actual pointer read from the retained handler's live vptr + 0x20, with
    /// owning module and complete live leaf code bytes validated against the exact
    /// original before normalization in the same admitted context.
    LiveSlotObservation(u64),
    /// ELF relocation lookup does not exclude runtime interposition or rewriting.
    OriginalImagePrediction(u64),
}

/// A genuine original reverse-map answer. None means an observed native -1,
/// while an omitted row means unobserved and must not become a native miss.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservedHashResolution {
    pub hash: OperationHash,
    pub runtime_id: Option<HandlerId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    ModuleIdentity,
    InvalidEvidenceIdentity,
    SlotIdentity(usize),
    UnsupportedBinding(u64),
    UnobservedLiveBinding(u64),
    DuplicateHash(OperationHash),
    InvalidHashResolution(OperationHash),
    UnobservedHash(OperationHash),
    UnresolvedSavedHash(OperationHash),
    NumericSlotOutOfBounds(HandlerId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Leaf {
    Base,
    MakeTradeRoute,
}

/// Resolved actual handler, constructed only from admitted registry observations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Handler {
    observed: ObservedHandler,
    leaf: Leaf,
}
impl Handler {
    pub fn identity(self) -> ObservedHandler {
        self.observed
    }

    /// Original BaseHandler (0x2d95660) returns true; MakeTradeRoute (0x2da0920)
    /// returns false. Both complete leaves perform no reads/writes/calls or RNG.
    pub fn can_be_interrupted(self) -> bool {
        self.leaf == Leaf::Base
    }
}

/// Complete ordered observed slot array plus explicitly observed reverse-map
/// answers. It never derives runtime IDs from definition collection positions.
/// Rebuild after any actual registry mutation; borrowed evidence is not live state.
#[derive(Debug)]
pub struct ObservedRegistry<'a> {
    identity: ObservationIdentity<'a>,
    slots: &'a [Option<ObservedHandler>],
    hashes: &'a [ObservedHashResolution],
}
impl<'a> ObservedRegistry<'a> {
    pub fn new(
        identity: ObservationIdentity<'a>,
        slots: &'a [Option<ObservedHandler>],
        hashes: &'a [ObservedHashResolution],
    ) -> Result<Self, Error> {
        if identity.gamecore_sha256 != GAME_SHA {
            return Err(Error::ModuleIdentity);
        }
        let valid_digest = |s: &str| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if !valid_digest(identity.configuration_sha256) || !valid_digest(identity.capture_sha256) {
            return Err(Error::InvalidEvidenceIdentity);
        }
        for (i, slot) in slots.iter().enumerate() {
            if let Some(handler) = slot {
                if handler.runtime_id.0 as u32 as usize != i {
                    return Err(Error::SlotIdentity(i));
                }
                Self::leaf(handler.interruption_target)?;
            }
        }
        for (i, row) in hashes.iter().enumerate() {
            if hashes[..i].iter().any(|r| r.hash == row.hash) {
                return Err(Error::DuplicateHash(row.hash));
            }
            if let Some(id) = row.runtime_id {
                // Native hash lookup delegates to GetHandler again. Only an
                // index-form result with an actual bounded slot is admitted.
                if id.0 as u32 >= 0x800 || id.0 as u32 as usize >= slots.len() {
                    return Err(Error::InvalidHashResolution(row.hash));
                }
            }
        }
        Ok(Self {
            identity,
            slots,
            hashes,
        })
    }

    pub fn identity(&self) -> ObservationIdentity<'a> {
        self.identity
    }

    fn leaf(target: InterruptionTarget) -> Result<Leaf, Error> {
        let rva = match target {
            InterruptionTarget::LiveSlotObservation(rva) => rva,
            InterruptionTarget::OriginalImagePrediction(rva) => {
                return Err(Error::UnobservedLiveBinding(rva));
            }
        };
        match rva {
            0x2d95660 => Ok(Leaf::Base),
            0x2da0920 => Ok(Leaf::MakeTradeRoute),
            _ => Err(Error::UnsupportedBinding(rva)),
        }
    }

    fn hash_resolution(&self, hash: OperationHash) -> Result<Option<HandlerId>, Error> {
        self.hashes
            .iter()
            .find(|r| r.hash == hash)
            .map(|r| r.runtime_id)
            .ok_or(Error::UnobservedHash(hash))
    }

    /// Resolve an owned wire queue hash. A missing native reverse-map entry is
    /// unsupported saved identity, not a fabricated current operation of -1.
    pub fn saved_hash_runtime_id(&self, hash: OperationHash) -> Result<HandlerId, Error> {
        self.hash_resolution(hash)?
            .ok_or(Error::UnresolvedSavedHash(hash))
    }

    /// Original GetHandler const: unsigned raw bits below 0x800 select a vector
    /// slot; all other values select the hash lookup. Numeric bounds are checked
    /// here to report unsupported input rather than imitate native invalid access.
    pub fn handler(&self, id: HandlerId) -> Result<Option<Handler>, Error> {
        let index = if (id.0 as u32) < 0x800 {
            id.0 as u32 as usize
        } else {
            match self.hash_resolution(OperationHash(id.0 as u32))? {
                Some(resolved) => resolved.0 as u32 as usize,
                None => return Ok(None),
            }
        };
        let slot = self
            .slots
            .get(index)
            .ok_or(Error::NumericSlotOutOfBounds(id))?;
        slot.map(|observed| {
            Ok(Handler {
                observed,
                leaf: Self::leaf(observed.interruption_target)?,
            })
        })
        .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity() -> ObservationIdentity<'static> {
        ObservationIdentity {
            gamecore_sha256: GAME_SHA,
            configuration_sha256:
                "aa38b5edfe6f4c5ffc8fc25d079e9739ab7e3d75d3998c723dbbdaece5e08aff",
            capture_sha256: "3fd5b6e64b5f81e06fc06187b9dae53691ddb536a6fc0b03f6f728d759dea585",
        }
    }
    // These manually supplied slot values exercise source semantics only. The
    // closed metadata capture has no live +0x20 pointers and cannot admit them.
    #[test]
    fn source_only_synthetic_trade_and_move_bindings_preserve_hash_index_distinctions() {
        let mut slots = [None; 20];
        slots[18] = Some(ObservedHandler {
            runtime_id: HandlerId(18),
            operation_hash: OperationHash(0x8374d954),
            interruption_target: InterruptionTarget::LiveSlotObservation(0x2da0920),
        });
        slots[19] = Some(ObservedHandler {
            runtime_id: HandlerId(19),
            operation_hash: OperationHash(0x580f2f68),
            interruption_target: InterruptionTarget::LiveSlotObservation(0x2d95660),
        });
        let hashes = [
            ObservedHashResolution {
                hash: OperationHash(0x8374d954),
                runtime_id: Some(HandlerId(18)),
            },
            ObservedHashResolution {
                hash: OperationHash(0x580f2f68),
                runtime_id: Some(HandlerId(19)),
            },
            ObservedHashResolution {
                hash: OperationHash(0xfffffff0),
                runtime_id: None,
            },
            // Even an observed small hash does not change GetHandler's index priority.
            ObservedHashResolution {
                hash: OperationHash(1),
                runtime_id: Some(HandlerId(18)),
            },
        ];
        let registry = ObservedRegistry::new(identity(), &slots, &hashes).unwrap();
        assert!(!registry
            .handler(HandlerId(18))
            .unwrap()
            .unwrap()
            .can_be_interrupted());
        assert!(!registry
            .handler(HandlerId(0x8374d954u32 as i32))
            .unwrap()
            .unwrap()
            .can_be_interrupted());
        assert!(registry
            .handler(HandlerId(19))
            .unwrap()
            .unwrap()
            .can_be_interrupted());
        assert_eq!(registry.handler(HandlerId(1)), Ok(None));
        assert_eq!(
            registry.saved_hash_runtime_id(OperationHash(1)),
            Ok(HandlerId(18))
        );
        assert_eq!(
            registry.handler(HandlerId(20)),
            Err(Error::NumericSlotOutOfBounds(HandlerId(20)))
        );
        assert_eq!(
            registry.handler(HandlerId(-1)),
            Err(Error::UnobservedHash(OperationHash(u32::MAX)))
        );
        assert_eq!(registry.handler(HandlerId(0xfffffff0u32 as i32)), Ok(None));
        assert_eq!(
            registry.saved_hash_runtime_id(OperationHash(0xfffffff0)),
            Err(Error::UnresolvedSavedHash(OperationHash(0xfffffff0)))
        );
    }
    #[test]
    fn admission_rejects_unverified_module_bindings_and_ambiguous_reverse_map() {
        let mut wrong = identity();
        wrong.gamecore_sha256 = "unknown";
        assert!(matches!(
            ObservedRegistry::new(wrong, &[], &[]),
            Err(Error::ModuleIdentity)
        ));
        let slots = [Some(ObservedHandler {
            runtime_id: HandlerId(0),
            operation_hash: OperationHash(55),
            interruption_target: InterruptionTarget::LiveSlotObservation(123),
        })];
        assert!(matches!(
            ObservedRegistry::new(identity(), &slots, &[]),
            Err(Error::UnsupportedBinding(123))
        ));
        let predictions = [Some(ObservedHandler {
            runtime_id: HandlerId(0),
            operation_hash: OperationHash(55),
            interruption_target: InterruptionTarget::OriginalImagePrediction(0x2d95660),
        })];
        assert!(matches!(
            ObservedRegistry::new(identity(), &predictions, &[]),
            Err(Error::UnobservedLiveBinding(0x2d95660))
        ));
        let hashes = [ObservedHashResolution {
            hash: OperationHash(55),
            runtime_id: None,
        }; 2];
        assert!(matches!(
            ObservedRegistry::new(identity(), &[], &hashes),
            Err(Error::DuplicateHash(OperationHash(55)))
        ));
        let hashes = [ObservedHashResolution {
            hash: OperationHash(55),
            runtime_id: Some(HandlerId(-1)),
        }];
        assert!(matches!(
            ObservedRegistry::new(identity(), &[], &hashes),
            Err(Error::InvalidHashResolution(OperationHash(55)))
        ));
    }
}
