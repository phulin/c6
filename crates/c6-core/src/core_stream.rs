//! Bounded byte-zero readers for current Linux XP2 raw Core prefixes.
//!
//! This is neither a `.Civ6Save` container reader nor native Context::Read: callers
//! must establish module/configuration provenance. Metadata values are preserved,
//! but native metadata compatibility, definition-index resolution, initialization,
//! RNG reconstruction and complete executable game state are not established.
//! The exact grammar is documented in `docs/linux-core-stream-layout.md` for
//! Linux module SHA-256 a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa.
//! Unknown unframed Rules providers fail. CorePrefix stops after Map; the
//! ThroughFirstPlayer composition continues through Religion and the first
//! Player's AI Culture, stopping before AI Diplomatic on the true saved branch.
//! The never-alive branch closes one saved-wire Player; an empty Manager also
//! has an explicit boundary. Neither result establishes executable world state.
//! Each result borrows its untouched remaining tail without semantic exclusion.

use crate::map::{self, Map};
use crate::unit::variants::{self, DecodeBudget, TypedVariantMap, VariantMap};

/// Matches Map's conservative charge for each variant item/name/string byte.
const VARIANT_ALLOCATION_PER_ITEM: usize = 256;
const PLAYER_SLOTS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    ResourceLimit,
    UnsupportedVersion {
        section: &'static str,
        expected: i32,
        actual: i32,
    },
    UnsupportedRulesProviderCount(u32),
    UnsupportedRulesProvider(u32),
    NoncanonicalBool(u8),
    Variant(variants::VariantError),
    Map(map::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl From<variants::VariantError> for Error {
    fn from(value: variants::VariantError) -> Self {
        Self::Variant(value)
    }
}
impl From<map::Error> for Error {
    fn from(value: map::Error) -> Self {
        Self::Map(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameProcessor {
    pub last_initial_ai: [i32; PLAYER_SLOTS],
    pub turn_order: [i32; PLAYER_SLOTS],
    pub last_ai_processed_turn: i32,
    pub turn_complete: bool,
    pub advance_turn: bool,
    pub new_game: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerProcessor {
    pub activation_event: [bool; PLAYER_SLOTS],
    pub first_activation: [bool; PLAYER_SLOTS],
    pub activation_needed: [bool; PLAYER_SLOTS],
    pub automation_processed: [bool; PLAYER_SLOTS],
    pub player_complete: [bool; PLAYER_SLOTS],
    pub game_complete: [bool; PLAYER_SLOTS],
    pub updates_this_phase: [u32; PLAYER_SLOTS],
    pub updates_this_turn: [u32; PLAYER_SLOTS],
    pub ai_updates_this_turn: [u32; PLAYER_SLOTS],
    pub active_count: u32,
    pub human_first_activated: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TurnSegment {
    pub hash: u32,
    pub disabled: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseProcessor {
    /// Raw native enum integer, not a SQL ordinal or a reconstructed phase.
    pub current_phase: i32,
    pub time_limit: u32,
    /// Original order, including any repeated hashes.
    pub turn_segments: Vec<TurnSegment>,
    pub last_valid_segment: i32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CorePrefix<'a> {
    pub metadata: TypedVariantMap,
    pub game_processor: GameProcessor,
    pub player_processor: PlayerProcessor,
    pub phase_processor: PhaseProcessor,
    pub current_turn: i32,
    pub max_turns: i32,
    pub local_player: i32,
    pub game_properties: VariantMap,
    pub map: Map,
    /// Absolute raw Core offsets; no save-container offsets are implied.
    pub map_start: usize,
    pub map_end: usize,
    /// Equal to map_end; the next unframed serializer is Game.Religion.
    pub consumed: usize,
    /// Aggregate item charge through Map, for readers continuing into the tail.
    pub items_used: usize,
    /// Aggregate logical allocation charge under the declared limits policy.
    /// Includes conservative variant charges; this is not native heap usage.
    pub logical_allocation_used: usize,
    /// Untouched input bytes. No tail copy or semantic exclusion is performed.
    pub tail: &'a [u8],
}
impl<'a> CorePrefix<'a> {
    /// Decode only supported current versions through complete Map.
    ///
    /// `max_bytes` bounds consumed prefix bytes, excluding the borrowed tail. `max_items` is aggregate over fixed processor-array elements, phase
    /// entries, variant entries/payload bytes and Map items. `max_allocation`
    /// bounds logical owned vectors plus conservative variant charges, excluding
    /// allocator overhead and inline fixed arrays, as in Map's caller policy.
    /// No module hash or metadata compatibility is inferred from these bytes.
    pub fn decode(bytes: &'a [u8], limits: map::Limits) -> Result<Self, Error> {
        let mut r = Reader {
            bytes,
            pos: 0,
            items: 0,
            allocation: 0,
            limits,
        };
        r.version("Context", 5)?;
        let metadata = r.variant(TypedVariantMap::decode_prefix)?;
        r.version("IndexSerializer", 1)?;
        r.version("Game", 22)?;
        r.version("Rules", 1)?;
        let count = r.u32()?;
        if count != 1 {
            return Err(Error::UnsupportedRulesProviderCount(count));
        }
        let provider = r.u32()?;
        if provider != 0xf3e6_941e {
            return Err(Error::UnsupportedRulesProvider(provider));
        }
        r.version("Rules.Placement", 1)?;
        r.version("GameProcessor", 2)?;
        let game_processor = GameProcessor {
            last_initial_ai: r.array(Reader::i32)?,
            turn_order: r.array(Reader::i32)?,
            last_ai_processed_turn: r.i32()?,
            turn_complete: r.boolean()?,
            advance_turn: r.boolean()?,
            new_game: r.boolean()?,
        };
        r.version("PlayerProcessor", 2)?;
        let player_processor = PlayerProcessor {
            activation_event: r.array(Reader::boolean)?,
            first_activation: r.array(Reader::boolean)?,
            activation_needed: r.array(Reader::boolean)?,
            automation_processed: r.array(Reader::boolean)?,
            player_complete: r.array(Reader::boolean)?,
            game_complete: r.array(Reader::boolean)?,
            updates_this_phase: r.array(Reader::u32)?,
            updates_this_turn: r.array(Reader::u32)?,
            ai_updates_this_turn: r.array(Reader::u32)?,
            active_count: r.u32()?,
            human_first_activated: r.boolean()?,
        };
        r.version("PhaseProcessor", 3)?;
        let current_phase = r.i32()?;
        let time_limit = r.u32()?;
        let count = r.u32()? as usize;
        if count > (bytes.len() - r.pos) / 5 {
            return Err(Error::Truncated);
        }
        r.charge(
            count,
            count
                .checked_mul(std::mem::size_of::<TurnSegment>())
                .ok_or(Error::ResourceLimit)?,
        )?;
        let mut turn_segments = Vec::new();
        turn_segments
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..count {
            turn_segments.push(TurnSegment {
                hash: r.u32()?,
                disabled: r.boolean()?,
            });
        }
        let phase_processor = PhaseProcessor {
            current_phase,
            time_limit,
            turn_segments,
            last_valid_segment: r.i32()?,
        };
        let current_turn = r.i32()?;
        let max_turns = r.i32()?;
        let local_player = r.i32()?;
        let game_properties = r.variant(VariantMap::decode_prefix)?;
        let map_start = r.pos;
        let (map, consumed, map_items, map_allocation) = Map::decode_prefix_with_usage(
            &bytes[map_start..],
            map::Limits {
                max_bytes: limits.max_bytes - map_start,
                max_items: limits.max_items - r.items,
                max_allocation: limits.max_allocation - r.allocation,
                max_depth: limits.max_depth,
            },
        )?;
        r.charge(map_items, map_allocation)?;
        r.take(consumed)?;
        Ok(Self {
            metadata,
            game_processor,
            player_processor,
            phase_processor,
            current_turn,
            max_turns,
            local_player,
            game_properties,
            map,
            map_start,
            map_end: r.pos,
            consumed: r.pos,
            items_used: r.items,
            logical_allocation_used: r.allocation,
            tail: &bytes[r.pos..],
        })
    }
}

/// Errors from byte-zero composition, retaining each component's exact refusal.
#[derive(Debug, Eq, PartialEq)]
pub enum ThroughFirstPlayerError {
    Core(Error),
    Player(crate::core_player::Error),
    ResourceLimit,
}
impl std::fmt::Display for ThroughFirstPlayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ThroughFirstPlayerError {}

/// Raw Core prefix through Game.Religion and the first Player's supported wire.
/// The true saved branch remains unfinished at AI Diplomatic after AI Culture;
/// the never-alive branch closes that one wire record. An empty Manager stops
/// at AfterEmptyPlayerManager. These boundaries do not establish a complete world.
#[derive(Debug, Eq, PartialEq)]
pub struct ThroughFirstPlayer<'a> {
    pub core: CorePrefix<'a>,
    pub player_prefix: crate::core_player::Prefix,
    /// Absolute raw Core cursor at player_prefix.boundary.
    pub consumed: usize,
    /// Aggregate charge across both decoded sections.
    pub items_used: usize,
    /// Untouched bytes beginning at the explicit continuation boundary.
    pub tail: &'a [u8],
}
impl<'a> ThroughFirstPlayer<'a> {
    /// Decode directly from byte zero without offsets, searches or opaque skips.
    ///
    /// Shared max_bytes/max_items cover both sections. The post-Map decoder's
    /// effective policy is the minimum of each remaining shared limit and its
    /// explicit player_limits ceiling. Shared max_allocation/max_depth apply
    /// through Map only. The later codecs retain their reviewed unit::Limits
    /// per-vector allocation checks and aggregate item bound; no exact aggregate
    /// allocation metric is claimed beyond Map. Both byte policies exclude the
    /// final borrowed tail. CorePrefix's provenance/metadata limitations apply.
    pub fn decode(
        bytes: &'a [u8],
        shared: map::Limits,
        player_limits: crate::unit::Limits,
    ) -> Result<Self, ThroughFirstPlayerError> {
        let core = CorePrefix::decode(bytes, shared).map_err(ThroughFirstPlayerError::Core)?;
        let effective = crate::unit::Limits {
            max_bytes: player_limits
                .max_bytes
                .min(shared.max_bytes - core.consumed),
            max_items: player_limits
                .max_items
                .min(shared.max_items - core.items_used),
        };
        let (player_prefix, player_bytes, player_items) =
            crate::core_player::Prefix::decode_prefix_with_items(core.tail, effective)
                .map_err(ThroughFirstPlayerError::Player)?;
        let consumed = core
            .consumed
            .checked_add(player_bytes)
            .ok_or(ThroughFirstPlayerError::ResourceLimit)?;
        let items_used = core
            .items_used
            .checked_add(player_items)
            .ok_or(ThroughFirstPlayerError::ResourceLimit)?;
        Ok(Self {
            core,
            player_prefix,
            consumed,
            items_used,
            tail: &bytes[consumed..],
        })
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    items: usize,
    allocation: usize,
    limits: map::Limits,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let bytes = self.bytes.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(bytes)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(self.u32()? as i32)
    }
    fn boolean(&mut self) -> Result<bool, Error> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(Error::NoncanonicalBool(n)),
        }
    }
    fn version(&mut self, section: &'static str, expected: i32) -> Result<(), Error> {
        let actual = self.i32()?;
        if actual == expected {
            Ok(())
        } else {
            Err(Error::UnsupportedVersion {
                section,
                expected,
                actual,
            })
        }
    }
    fn charge(&mut self, items: usize, allocation: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(items).ok_or(Error::ResourceLimit)?;
        self.allocation = self
            .allocation
            .checked_add(allocation)
            .ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items || self.allocation > self.limits.max_allocation {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn array<T: Default + Copy>(
        &mut self,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<[T; PLAYER_SLOTS], Error> {
        self.charge(PLAYER_SLOTS, 0)?;
        let mut values = [T::default(); PLAYER_SLOTS];
        for value in &mut values {
            *value = read(self)?;
        }
        Ok(values)
    }
    fn variant<T>(
        &mut self,
        read: impl FnOnce(&[u8], &mut DecodeBudget) -> Result<(T, usize), variants::VariantError>,
    ) -> Result<T, Error> {
        let max_items = (self.limits.max_items - self.items)
            .min((self.limits.max_allocation - self.allocation) / VARIANT_ALLOCATION_PER_ITEM);
        let mut budget = DecodeBudget::new(variants::Limits {
            max_bytes: self.limits.max_bytes - self.pos,
            max_items,
            max_depth: self.limits.max_depth,
        });
        let (value, consumed) = read(&self.bytes[self.pos..], &mut budget)?;
        self.charge(
            budget.items_used(),
            budget
                .items_used()
                .checked_mul(VARIANT_ALLOCATION_PER_ITEM)
                .ok_or(Error::ResourceLimit)?,
        )?;
        self.take(consumed)?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use variants::{ArrayVariant, TypedEntry, TypedVariant, VariantPayload};

    fn words(bytes: &mut Vec<u8>, values: &[u32]) {
        for value in values {
            bytes.extend(value.to_le_bytes());
        }
    }
    // Source-only empty-world framing fixture, not an initialized native world.
    fn map_wire() -> Vec<u8> {
        let mut b = Vec::new();
        words(&mut b, &[4, 0, 0, 90, (-90i32) as u32]);
        b.extend([0, 0]);
        words(&mut b, &[0xdeadbeef, 0, 0, 0, 0, 0, 1, 0]);
        for i in 0..5 {
            if i != 3 {
                words(&mut b, &[1]);
            }
            words(&mut b, &[65537, 16, 0, 0, 0, 0, u32::MAX, u32::MAX]);
            if i == 3 {
                words(&mut b, &[0]);
            } // Continents before FeaturePlots.
        }
        words(&mut b, &[6, 0, 1, 0, 0, 0, 1, 1]);
        let name = b"TerritoryBuilder";
        words(&mut b, &[name.len() as u32]);
        b.extend(name);
        words(&mut b, &[0]);
        Map::decode(&b, map::Limits::default()).unwrap();
        b
    }
    fn metadata() -> TypedVariantMap {
        TypedVariantMap {
            entries: vec![TypedEntry {
                key: 0x12345678,
                name: None,
                value: TypedVariant {
                    type_bits: 1,
                    modifier_bits: 0,
                    value: ArrayVariant {
                        count_flags: 0,
                        element_size: 0,
                        payload: VariantPayload::Inline32(0xfedcba98),
                    },
                },
            }],
            ..TypedVariantMap::default()
        }
    }
    fn fixture(metadata: &TypedVariantMap, properties: &VariantMap) -> (Vec<u8>, usize, usize) {
        let mut b = Vec::new();
        words(&mut b, &[5]);
        b.extend(metadata.encode(variants::Limits::default()).unwrap().0);
        words(&mut b, &[1, 22, 1, 1, 0xf3e6941e, 1, 2]);
        for i in 0..128 {
            words(&mut b, &[0x80000000 + i]);
        }
        words(&mut b, &[u32::MAX]);
        b.extend([1, 0, 1]);
        words(&mut b, &[2]);
        for i in 0..384 {
            b.push((i % 3 == 0) as u8);
        }
        for i in 0..192 {
            words(&mut b, &[0xf0000000 + i]);
        }
        words(&mut b, &[7]);
        b.push(1);
        words(&mut b, &[3, u32::MAX, 0xfedcba98, 2, 0xabcdef01]);
        b.push(0);
        words(&mut b, &[0xabcdef01]);
        b.push(1); // Preserve repeated hashes and wire order.
        words(&mut b, &[0x80000000, 17, 500, u32::MAX]);
        b.extend(properties.encode(variants::Limits::default()).unwrap().0);
        let start = b.len();
        b.extend(map_wire());
        let end = b.len();
        b.extend([0xff, 0, 0xa5, 0x5a]); // Deliberately unsupported untouched bytes.
        (b, start, end)
    }
    #[test]
    fn composition_relocates_map_and_borrows_tail_preserving_processor_bits() {
        let empty = fixture(&TypedVariantMap::default(), &VariantMap::default());
        let properties = VariantMap {
            scalars: vec![(1, 0xffffffff), (9, 0x80000000)],
            arrays: vec![(
                3,
                ArrayVariant {
                    count_flags: 0,
                    element_size: 0,
                    payload: VariantPayload::Inline32(0x12345678),
                },
            )],
        };
        let (bytes, start, end) = fixture(&metadata(), &properties);
        let decoded = CorePrefix::decode(&bytes, map::Limits::default()).unwrap();
        assert_eq!(decoded.metadata, metadata());
        assert_eq!(decoded.game_properties, properties);
        assert_eq!(
            (decoded.map_start, decoded.map_end, decoded.consumed),
            (start, end, end)
        );
        assert_eq!(&bytes[start..end], &empty.0[empty.1..empty.2]);
        assert_eq!(start - empty.1, 52);
        assert!(std::ptr::eq(decoded.tail.as_ptr(), bytes[end..].as_ptr()));
        assert_eq!(decoded.tail, [0xff, 0, 0xa5, 0x5a]);
        assert_eq!(decoded.game_processor.last_initial_ai[0], i32::MIN);
        assert_eq!(decoded.game_processor.turn_order[63], i32::MIN + 127);
        assert_eq!(decoded.game_processor.last_ai_processed_turn, -1);
        assert!(decoded.game_processor.turn_complete && decoded.game_processor.new_game);
        assert!(!decoded.game_processor.advance_turn);
        assert!(decoded.player_processor.activation_event[0]);
        assert!(!decoded.player_processor.first_activation[0]);
        assert_eq!(
            decoded.player_processor.ai_updates_this_turn[63],
            0xf00000bf
        );
        assert_eq!(decoded.player_processor.active_count, 7);
        assert_eq!(decoded.phase_processor.current_phase, -1);
        assert_eq!(decoded.phase_processor.last_valid_segment, i32::MIN);
        assert_eq!(
            decoded
                .phase_processor
                .turn_segments
                .iter()
                .map(|s| s.disabled)
                .collect::<Vec<_>>(),
            [false, true]
        );
        assert_eq!(
            (
                decoded.current_turn,
                decoded.max_turns,
                decoded.local_player
            ),
            (17, 500, -1)
        );
        assert_eq!(
            Map::decode(&bytes[start..], map::Limits::default()),
            Err(map::Error::TrailingBytes)
        );
    }
    #[test]
    fn every_truncated_prefix_and_unknown_versions_providers_bools_reject() {
        let (bytes, _, end) = fixture(&TypedVariantMap::default(), &VariantMap::default());
        for cut in 0..end {
            assert!(
                CorePrefix::decode(&bytes[..cut], map::Limits::default()).is_err(),
                "cut {cut}"
            );
        }
        assert!(CorePrefix::decode(&bytes[..end], map::Limits::default())
            .unwrap()
            .tail
            .is_empty());
        for (offset, replacement) in [(0, 6), (8, 2), (12, 23), (16, 2), (28, 2), (32, 3)] {
            let mut bad = bytes.clone();
            bad[offset..offset + 4].copy_from_slice(&(replacement as u32).to_le_bytes());
            assert!(matches!(
                CorePrefix::decode(&bad, map::Limits::default()),
                Err(Error::UnsupportedVersion { .. })
            ));
        }
        let mut bad = bytes.clone();
        bad[20..24].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            CorePrefix::decode(&bad, map::Limits::default()),
            Err(Error::UnsupportedRulesProviderCount(2))
        );
        bad[20..24].copy_from_slice(&1u32.to_le_bytes());
        bad[24..28].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            CorePrefix::decode(&bad, map::Limits::default()),
            Err(Error::UnsupportedRulesProvider(0))
        );
        let mut bad = bytes;
        bad[36 + 128 * 4 + 4] = 2;
        assert_eq!(
            CorePrefix::decode(&bad, map::Limits::default()),
            Err(Error::NoncanonicalBool(2))
        );
    }
    #[test]
    fn core_composition_preserves_strict_map_component_refusals() {
        let (mut bytes, start, end) = fixture(&TypedVariantMap::default(), &VariantMap::default());
        // The final framed component has an empty payload and a 16-byte name.
        // Unknown names remain errors even though their framing could be skipped.
        bytes[end - 20..end - 4].copy_from_slice(b"UnknownComponent");
        assert_eq!(
            Map::decode(&bytes[start..end], map::Limits::default()),
            Err(map::Error::UnsupportedComponent)
        );
        assert_eq!(
            CorePrefix::decode(&bytes, map::Limits::default()),
            Err(Error::Map(map::Error::UnsupportedComponent))
        );
    }

    fn through_player_fixture(nonempty_religion: bool) -> (Vec<u8>, usize, usize) {
        let (mut bytes, _, map_end) = fixture(&metadata(), &VariantMap::default());
        bytes.truncate(map_end);
        words(
            &mut bytes,
            &[7, 0x80000000, u32::MAX, nonempty_religion as u32],
        );
        if nonempty_religion {
            words(
                &mut bytes,
                &[7, 0x12345678, u32::MAX, 11, 0x80000000, 90, 256],
            );
            bytes.extend([0xa5; 256]);
            words(
                &mut bytes,
                &[1, 0xabcdef01, 2, 0xdeadbeef, 0x87654321, 0, 17],
            );
            bytes.extend([1, 0, 1]);
            words(&mut bytes, &[0x80000000]);
        }
        words(&mut bytes, &[1, 0]); // Empty Player.Manager, no Player can be skipped.
        let end = bytes.len();
        bytes.extend([0xff, 0xa5, 0x5a]);
        (bytes, map_end, end)
    }
    #[test]
    fn byte_zero_composition_retains_nonempty_religion_and_explicit_empty_manager_boundary() {
        for nonempty in [false, true] {
            let (bytes, map_end, end) = through_player_fixture(nonempty);
            let decoded = ThroughFirstPlayer::decode(
                &bytes,
                map::Limits::default(),
                crate::unit::Limits::default(),
            )
            .unwrap();
            assert_eq!(decoded.core.consumed, map_end);
            assert_eq!(decoded.consumed, end);
            assert_eq!(decoded.player_prefix.player_count, 0);
            assert_eq!(decoded.player_prefix.first_player, None);
            assert_eq!(
                decoded.player_prefix.boundary,
                crate::core_player::Boundary::AfterEmptyPlayerManager
            );
            assert_eq!(
                decoded.player_prefix.religion.records.len(),
                nonempty as usize
            );
            assert!(std::ptr::eq(decoded.tail.as_ptr(), bytes[end..].as_ptr()));
            assert!(std::ptr::eq(
                decoded.core.tail.as_ptr(),
                bytes[map_end..].as_ptr()
            ));
            assert_eq!(decoded.tail, [0xff, 0xa5, 0x5a]);
            if nonempty {
                let religion = &decoded.player_prefix.religion.records[0];
                assert_eq!(religion.religion_hash, 0x12345678);
                assert_eq!(religion.custom_name, [0xa5; 256]);
                assert_eq!(religion.building_hashes, [0xdeadbeef, 0x87654321]);
                assert!(religion.condemn_prohibited && religion.awards_envoy);
                assert!(!religion.auto_spreads_to_new_cities);
                assert_eq!(religion.awards_envoy_religious_pressure, i32::MIN);
            }
            for cut in map_end..end {
                assert!(
                    ThroughFirstPlayer::decode(
                        &bytes[..cut],
                        map::Limits::default(),
                        crate::unit::Limits::default()
                    )
                    .is_err(),
                    "post-Map cut {cut}"
                );
            }
            let mut unknown = bytes;
            unknown[map_end..map_end + 4].copy_from_slice(&8u32.to_le_bytes());
            assert!(matches!(
                ThroughFirstPlayer::decode(
                    &unknown,
                    map::Limits::default(),
                    crate::unit::Limits::default()
                ),
                Err(ThroughFirstPlayerError::Player(
                    crate::core_player::Error::Version {
                        expected: 7,
                        actual: 8
                    }
                ))
            ));
        }
    }
    #[test]
    fn byte_zero_composition_carries_shared_and_explicit_player_budget_caps() {
        let (mut bytes, map_end, end) = through_player_fixture(true);
        let base = CorePrefix::decode(&bytes, map::Limits::default()).unwrap();
        // One Religion record, 256 name bytes, one belief and two building hashes.
        let player_items = 1 + 256 + 1 + 2;
        let total_items = base.items_used + player_items;
        let shared = map::Limits {
            max_bytes: end,
            max_items: total_items,
            ..map::Limits::default()
        };
        let player = crate::unit::Limits {
            max_bytes: end - map_end,
            max_items: player_items,
        };
        bytes.resize(end + 1_000_000, 0xa5);
        let decoded = ThroughFirstPlayer::decode(&bytes, shared, player).unwrap();
        assert_eq!((decoded.consumed, decoded.items_used), (end, total_items));
        assert!(std::ptr::eq(decoded.tail.as_ptr(), bytes[end..].as_ptr()));
        for limited in [
            map::Limits {
                max_bytes: end - 1,
                ..shared
            },
            map::Limits {
                max_items: total_items - 1,
                ..shared
            },
        ] {
            // Both Core-only budgets would pass: failure must come from carrying
            // their remaining budgets into the post-Map decoder.
            assert!(CorePrefix::decode(&bytes, limited).is_ok());
            assert!(matches!(
                ThroughFirstPlayer::decode(&bytes, limited, crate::unit::Limits::default()),
                Err(ThroughFirstPlayerError::Player(
                    crate::core_player::Error::ResourceLimit
                ))
            ));
        }
        for limited in [
            crate::unit::Limits {
                max_bytes: player.max_bytes - 1,
                ..player
            },
            crate::unit::Limits {
                max_items: player.max_items - 1,
                ..player
            },
        ] {
            assert!(matches!(
                ThroughFirstPlayer::decode(&bytes, map::Limits::default(), limited),
                Err(ThroughFirstPlayerError::Player(
                    crate::core_player::Error::ResourceLimit
                ))
            ));
        }
    }

    #[test]
    fn budgets_aggregate_across_variants_phase_and_map_excluding_borrowed_tail() {
        let properties = VariantMap {
            scalars: vec![(1, 2)],
            arrays: vec![],
        };
        let (mut bytes, _, end) = fixture(&metadata(), &properties);
        bytes.resize(end + 1_000_000, 0xa5);
        // 704 static entries, two variants, two phase entries, one Map component.
        let exact_items = 704 + 2 + 2 + 1;
        let exact_allocation =
            512 + 2 * std::mem::size_of::<TurnSegment>() + std::mem::size_of::<map::Component>();
        let limits = map::Limits {
            max_bytes: end,
            max_items: exact_items,
            max_allocation: exact_allocation,
            ..map::Limits::default()
        };
        let decoded = CorePrefix::decode(&bytes, limits).unwrap();
        assert_eq!(decoded.consumed, end);
        assert_eq!(decoded.items_used, exact_items);
        assert_eq!(decoded.logical_allocation_used, exact_allocation);
        assert!(std::ptr::eq(decoded.tail.as_ptr(), bytes[end..].as_ptr()));
        // An enclosing reader can subtract these exact charges before decoding
        // subsequent records; no fresh budget or allocation estimate is needed.
        assert_eq!(limits.max_items - decoded.items_used, 0);
        assert_eq!(limits.max_allocation - decoded.logical_allocation_used, 0);
        for limited in [
            map::Limits {
                max_bytes: end - 1,
                ..limits
            },
            map::Limits {
                max_items: exact_items - 1,
                ..limits
            },
            map::Limits {
                max_allocation: exact_allocation - 1,
                ..limits
            },
        ] {
            assert!(CorePrefix::decode(&bytes, limited).is_err());
        }
        let (bytes, _, _) = fixture(&metadata(), &VariantMap::default());
        assert!(matches!(
            CorePrefix::decode(
                &bytes,
                map::Limits {
                    max_allocation: 255,
                    ..map::Limits::default()
                }
            ),
            Err(Error::Variant(variants::VariantError::Limit))
        ));
    }
}
