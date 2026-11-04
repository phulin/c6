//! Game.Quests2 wire projection, separated at native parameter dispatch.
//! QuestData2 does not serialize inline primary/secondary parameters. Their
//! prior values are unknown here. Parameter words require an authoritative
//! Quest-definition/registered-logic catalog; definition hashes are never indices.
//! No native registration, initialization, callbacks or quest gameplay runs.

pub use crate::unit::Limits;
pub const VERSION: u32 = 2;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
    MissingQuestCatalog { type_hash: u32 },
    UnsupportedRegisteredLogic { type_hash: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Game.Quests wire: {self:?}")
    }
}
impl std::error::Error for Error {}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl Reader<'_> {
    fn word(&mut self) -> Result<u32, Error> {
        let end = self.pos.checked_add(4).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let bytes = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
    }
    fn version(&mut self) -> Result<u32, Error> {
        let actual = self.word()?;
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        Ok(actual)
    }
    fn quests(&mut self) -> Result<Vec<QuestData>, Error> {
        let count = self.word()? as usize;
        self.items = self.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        let minimum = count.checked_mul(32).ok_or(Error::ResourceLimit)?;
        if minimum > self.limits.max_bytes - self.pos {
            return Err(Error::ResourceLimit);
        }
        if minimum > self.data.len() - self.pos {
            return Err(Error::Truncated);
        }
        count
            .checked_mul(std::mem::size_of::<QuestData>())
            .ok_or(Error::ResourceLimit)?;
        let mut quests = Vec::new();
        quests
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..count {
            quests.push(QuestData {
                wire_version: self.version()?,
                state_ordinal: self.word()? as i32,
                client_ordinal: self.word()? as i32,
                actor_ordinal: self.word()? as i32,
                type_hash: self.word()?,
                id: self.word()? as i32,
                turn_started: self.word()? as i32,
                turn_completed: self.word()? as i32,
            });
        }
        Ok(quests)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestData {
    pub wire_version: u32,
    pub state_ordinal: i32,
    pub client_ordinal: i32,
    pub actor_ordinal: i32,
    pub type_hash: u32,
    pub id: i32,
    pub turn_started: i32,
    pub turn_completed: i32,
}
/// Both fixed lists, stopping before every per-quest parameter reader. This is
/// not the complete Manager extent. Inline primary/secondary prior state is
/// deliberately not synthesized from omitted QuestData2 bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedPrefix {
    pub wire_version: u32,
    pub num_quests_ever_assigned: i32,
    pub current: Vec<QuestData>,
    pub completed: Vec<QuestData>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuestLogic {
    ConvertCapitalToReligion,
    SendTradeRoute,
    ClearBarbarianCamp,
    TrainUnitType,
    ZoneDistrictType,
    TriggerTechBoost,
    TriggerCivicBoost,
    RecruitGreatPersonClass,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeResolution {
    /// An authoritative native Quest hash lookup returned-1, which skips dispatch.
    MissingNativeType,
    Registered(QuestLogic),
}
/// The caller must supply admitted Quest definition hash resolution and the source-bound
/// registered class. Unknown input must return MissingQuestCatalog; an unbound
/// SQL row or an unobserved registry does not establish MissingNativeType.
pub trait QuestTypeCatalog {
    fn resolve(&self, type_hash: u32) -> Result<TypeResolution, Error>;
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterDefinition {
    Unit,
    District,
    Tech,
    Civic,
    GreatPersonClass,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterWire {
    MissingNativeType,
    /// The proven registered reader consumes0 bytes. Prior parameters remain
    /// unknown; this variant does not substitute a constructed quest object.
    NoSerializedParameter {
        logic: QuestLogic,
    },
    SignedPrimary {
        header_raw: i32,
        value: i32,
    },
    DefinitionHashPrimary {
        header_raw: i32,
        kind: ParameterDefinition,
        definition_hash: u32,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterTail {
    /// Same occurrence order as each fixed list. No ID/key deduplication.
    pub current: Vec<ParameterWire>,
    pub completed: Vec<ParameterWire>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sequence {
    Current,
    Completed,
}
/// Rust refusal cursor relative to the parameter section, not a native PC.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParameterBoundary {
    pub consumed: usize,
    pub sequence: Sequence,
    pub index: usize,
    pub type_hash: u32,
    pub error: Error,
}
fn parameter(r: &mut Reader<'_>, resolution: TypeResolution) -> Result<ParameterWire, Error> {
    let TypeResolution::Registered(logic) = resolution else {
        return Ok(ParameterWire::MissingNativeType);
    };
    use QuestLogic::*;
    if matches!(logic, SendTradeRoute | ClearBarbarianCamp) {
        return Ok(ParameterWire::NoSerializedParameter { logic });
    }
    // The original readers consume/discard this signed word without branching
    // on it. Preserve it instead of imposing a fabricated version gate.
    let header_raw = r.word()? as i32;
    let value = r.word()?;
    if logic == ConvertCapitalToReligion {
        return Ok(ParameterWire::SignedPrimary {
            header_raw,
            value: value as i32,
        });
    }
    let kind = match logic {
        TrainUnitType => ParameterDefinition::Unit,
        ZoneDistrictType => ParameterDefinition::District,
        TriggerTechBoost => ParameterDefinition::Tech,
        TriggerCivicBoost => ParameterDefinition::Civic,
        RecruitGreatPersonClass => ParameterDefinition::GreatPersonClass,
        _ => unreachable!(),
    };
    Ok(ParameterWire::DefinitionHashPrimary {
        header_raw,
        kind,
        definition_hash: value,
    })
}
impl FixedPrefix {
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let mut r = Reader {
            data: bytes,
            pos: 0,
            items: 0,
            limits,
        };
        let value = Self {
            wire_version: r.version()?,
            num_quests_ever_assigned: r.word()? as i32,
            current: r.quests()?,
            completed: r.quests()?,
        };
        Ok((value, r.pos, r.items))
    }
    /// Parse only the subsequent current-then-completed parameter section.
    /// Its scalar words add0 counted wire items; metadata vectors reuse the
    /// occurrences already charged by the fixed lists. Secondary prior values
    /// and native hash-to-index conversion remain unavailable.
    pub fn decode_parameter_tail(
        &self,
        bytes: &[u8],
        limits: Limits,
        catalog: &impl QuestTypeCatalog,
    ) -> Result<(ParameterTail, usize), ParameterBoundary> {
        if self.wire_version != VERSION {
            return Err(ParameterBoundary {
                consumed: 0,
                sequence: Sequence::Current,
                index: 0,
                type_hash: 0,
                error: Error::Version {
                    expected: VERSION,
                    actual: self.wire_version,
                },
            });
        }
        for (sequence, quests) in [
            (Sequence::Current, &self.current),
            (Sequence::Completed, &self.completed),
        ] {
            for (index, quest) in quests.iter().enumerate() {
                if quest.wire_version != VERSION {
                    return Err(ParameterBoundary {
                        consumed: 0,
                        sequence,
                        index,
                        type_hash: quest.type_hash,
                        error: Error::Version {
                            expected: VERSION,
                            actual: quest.wire_version,
                        },
                    });
                }
            }
        }
        let mut r = Reader {
            data: bytes,
            pos: 0,
            items: 0,
            limits,
        };
        let mut results = [Vec::new(), Vec::new()];
        for ((sequence, quests), output) in [
            (Sequence::Current, &self.current),
            (Sequence::Completed, &self.completed),
        ]
        .into_iter()
        .zip(&mut results)
        {
            for (index, quest) in quests.iter().enumerate() {
                let boundary = |error| ParameterBoundary {
                    consumed: r.pos,
                    sequence,
                    index,
                    type_hash: quest.type_hash,
                    error,
                };
                if index == 0 {
                    quests
                        .len()
                        .checked_mul(std::mem::size_of::<ParameterWire>())
                        .ok_or_else(|| boundary(Error::ResourceLimit))?;
                    output
                        .try_reserve_exact(quests.len())
                        .map_err(|_| boundary(Error::ResourceLimit))?;
                }
                let resolution = catalog.resolve(quest.type_hash).map_err(boundary)?;
                let value = parameter(&mut r, resolution).map_err(|error| ParameterBoundary {
                    consumed: r.pos,
                    sequence,
                    index,
                    type_hash: quest.type_hash,
                    error,
                })?;
                output.push(value);
            }
        }
        let [current, completed] = results;
        Ok((ParameterTail { current, completed }, r.pos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn words(b: &mut Vec<u8>, v: &[u32]) {
        for n in v {
            b.extend_from_slice(&n.to_le_bytes());
        }
    }
    fn record(b: &mut Vec<u8>, kind: u32, id: u32) {
        words(
            b,
            &[
                2,
                u32::MAX,
                1,
                i32::MIN as u32,
                kind,
                id,
                27,
                i32::MAX as u32,
            ],
        );
    }
    // Synthetic hash labels are not native enum indices, SQL IDs or registry evidence.
    struct Catalog;
    impl QuestTypeCatalog for Catalog {
        fn resolve(&self, hash: u32) -> Result<TypeResolution, Error> {
            use QuestLogic::*;
            let kinds = [
                ConvertCapitalToReligion,
                SendTradeRoute,
                ClearBarbarianCamp,
                TrainUnitType,
                ZoneDistrictType,
                TriggerTechBoost,
                TriggerCivicBoost,
                RecruitGreatPersonClass,
            ];
            match hash {
                0..=7 => Ok(TypeResolution::Registered(kinds[hash as usize])),
                8 => Ok(TypeResolution::MissingNativeType),
                _ => Err(Error::MissingQuestCatalog { type_hash: hash }),
            }
        }
    }
    #[test]
    fn typed_unsupported_context_refuses_before_catalog_or_parameter_reads() {
        struct Never;
        impl QuestTypeCatalog for Never {
            fn resolve(&self, _: u32) -> Result<TypeResolution, Error> {
                panic!("unsupported context must refuse first")
            }
        }
        let mut bytes = Vec::new();
        words(&mut bytes, &[2, 1, 1]);
        record(&mut bytes, 3, 1);
        words(&mut bytes, &[0]);
        let (mut p, _, _) =
            FixedPrefix::decode_prefix_with_items(&bytes, Limits::default()).unwrap();
        p.wire_version = 1;
        let e = p
            .decode_parameter_tail(&[], Limits::default(), &Never)
            .unwrap_err();
        assert_eq!(
            (e.consumed, e.error),
            (
                0,
                Error::Version {
                    expected: 2,
                    actual: 1
                }
            )
        );
        p.wire_version = 2;
        p.current[0].wire_version = 1;
        let e = p
            .decode_parameter_tail(&[], Limits::default(), &Never)
            .unwrap_err();
        assert_eq!(
            (e.consumed, e.index, e.error),
            (
                0,
                0,
                Error::Version {
                    expected: 2,
                    actual: 1
                }
            )
        );
    }
    #[test]
    fn fixed_lists_stop_before_parameter_payload_and_keep_unserialized_prior_unknown() {
        let mut b = Vec::new();
        words(&mut b, &[2, 650, 1]);
        record(&mut b, 4, 650);
        words(&mut b, &[1]);
        record(&mut b, 7, u32::MAX);
        b.push(0xaa);
        let cap = Limits {
            max_bytes: 80,
            max_items: 2,
        };
        let (p, n, i) = FixedPrefix::decode_prefix_with_items(&b, cap).unwrap();
        assert_eq!(
            (p.wire_version, p.num_quests_ever_assigned, n, i),
            (2, 650, 80, 2)
        );
        assert_eq!(
            (
                p.current[0].state_ordinal,
                p.current[0].actor_ordinal,
                p.completed[0].id
            ),
            (-1, i32::MIN, -1)
        );
        for cut in 0..80 {
            assert!(FixedPrefix::decode_prefix_with_items(&b[..cut], cap).is_err());
        }
        assert_eq!(
            FixedPrefix::decode_prefix_with_items(
                &b,
                Limits {
                    max_bytes: 79,
                    ..cap
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            FixedPrefix::decode_prefix_with_items(
                &b,
                Limits {
                    max_items: 1,
                    ..cap
                }
            ),
            Err(Error::ResourceLimit)
        );
        let mut bad = b.clone();
        bad[12..16].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            FixedPrefix::decode_prefix_with_items(&bad, cap),
            Err(Error::Version {
                expected: 2,
                actual: 1
            })
        );
        let mut bad = b;
        bad[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(FixedPrefix::decode_prefix_with_items(
            &bad,
            Limits {
                max_bytes: 80,
                max_items: usize::MAX
            }
        )
        .is_err());
    }
    #[test]
    fn eight_real_logic_shapes_preserve_order_headers_and_hash_kinds() {
        let mut b = Vec::new();
        words(&mut b, &[2, 9, 9]);
        for kind in 0..9 {
            record(&mut b, kind, kind);
        }
        words(&mut b, &[1]);
        record(&mut b, 3, 3);
        let (p, n, i) = FixedPrefix::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (336, 10));
        let mut tail = Vec::new();
        for kind in [0u32, 3, 4, 5, 6, 7, 3] {
            words(&mut tail, &[u32::MAX, 0x80000000 | kind]);
        }
        let (t, used) = p
            .decode_parameter_tail(
                &tail,
                Limits {
                    max_bytes: 56,
                    max_items: 0,
                },
                &Catalog,
            )
            .unwrap();
        assert_eq!(used, 56);
        assert_eq!(
            t.current[0],
            ParameterWire::SignedPrimary {
                header_raw: -1,
                value: i32::MIN
            }
        );
        assert_eq!(
            t.current[1],
            ParameterWire::NoSerializedParameter {
                logic: QuestLogic::SendTradeRoute
            }
        );
        assert_eq!(
            t.current[2],
            ParameterWire::NoSerializedParameter {
                logic: QuestLogic::ClearBarbarianCamp
            }
        );
        assert_eq!(t.current[8], ParameterWire::MissingNativeType);
        assert_eq!(
            t.current[3],
            ParameterWire::DefinitionHashPrimary {
                header_raw: -1,
                kind: ParameterDefinition::Unit,
                definition_hash: 0x80000003
            }
        );
        assert_eq!(t.completed[0], t.current[3]);
        for cut in 0..56 {
            assert!(p
                .decode_parameter_tail(&tail[..cut], Limits::default(), &Catalog)
                .is_err());
        }
        assert_eq!(
            p.decode_parameter_tail(
                &tail,
                Limits {
                    max_bytes: 55,
                    max_items: 0
                },
                &Catalog
            )
            .unwrap_err()
            .error,
            Error::ResourceLimit
        );
        let mut unknown = p;
        unknown.current[1].type_hash = 99;
        let e = unknown
            .decode_parameter_tail(&tail, Limits::default(), &Catalog)
            .unwrap_err();
        assert_eq!(
            (e.consumed, e.sequence, e.index, e.error),
            (
                8,
                Sequence::Current,
                1,
                Error::MissingQuestCatalog { type_hash: 99 }
            )
        );
    }
}
