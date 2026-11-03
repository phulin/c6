//! Current Linux XP2 Goody_Hut, Bonuses and Congress wire components.
//!
//! Ordered wire pairs remain distinct from native-registry read application.
//! This is not a Player codec, constructor for all Player state, or gameplay.
use crate::movement_world::DefinitionLookup;
use crate::unit::{HashValue, Limits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Version {
        expected: i32,
        actual: i32,
    },
    InvalidBool,
    Trailing,
    ResourceLimit,
    DefinitionMapping,
    MissingGovernmentSlots,
    DestinationSize,
    /// Native resize(int) does not initialize the new int values. No default
    /// is invented when neither existing storage nor a wire pair supplied it.
    MissingResizeValue {
        index: usize,
    },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player component input: {self:?}")
    }
}
impl std::error::Error for Error {}

fn reserve<T>(values: &mut Vec<T>, count: usize, limits: Limits) -> Result<(), Error> {
    if count > limits.max_items
        || count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::ResourceLimit)?
            > limits.max_bytes
    {
        return Err(Error::ResourceLimit);
    }
    values
        .try_reserve_exact(count)
        .map_err(|_| Error::ResourceLimit)
}
fn check_pairs(pairs: &[HashValue<i32>], limits: Limits) -> Result<(), Error> {
    if pairs.len() > limits.max_items
        || pairs.len().checked_mul(8).ok_or(Error::ResourceLimit)? > limits.max_bytes
    {
        return Err(Error::ResourceLimit);
    }
    Ok(())
}
fn index(key: u32, lookup: &dyn DefinitionLookup) -> Result<Option<usize>, Error> {
    if lookup.count() > i32::MAX as usize {
        return Err(Error::DefinitionMapping);
    }
    match lookup.find_index(key) {
        Some(i) if i < lookup.count() => Ok(Some(i)),
        Some(_) => Err(Error::DefinitionMapping),
        None => Ok(None),
    }
}

/// Original ReadIndexHashArray overlay: preserve unspecified values, discard
/// unknown hashes, and let later duplicate pairs overwrite. Requires complete
/// current destination storage; the wire count does not size that storage.
pub fn apply_indexed_ints(
    pairs: &[HashValue<i32>],
    prior: &[i32],
    lookup: &dyn DefinitionLookup,
    limits: Limits,
) -> Result<Vec<i32>, Error> {
    check_pairs(pairs, limits)?;
    if prior.len() != lookup.count() {
        return Err(Error::DestinationSize);
    }
    let mut values = Vec::new();
    reserve(&mut values, prior.len(), limits)?;
    values.extend_from_slice(prior);
    for pair in pairs {
        if let Some(i) = index(pair.definition_hash, lookup)? {
            values[i] = pair.value;
        }
    }
    Ok(values)
}
/// Only for the proven Reset paths: clear then resize(count, explicit int0).
/// It is not the default for a generic ReadIndexHashArray call.
pub fn reset_indexed_ints(
    lookup: &dyn DefinitionLookup,
    limits: Limits,
) -> Result<Vec<i32>, Error> {
    if lookup.count() > i32::MAX as usize {
        return Err(Error::DefinitionMapping);
    }
    let mut values = Vec::new();
    reserve(&mut values, lookup.count(), limits)?;
    values.resize(lookup.count(), 0);
    Ok(values)
}
/// Congress Read resizes to the actual count without initializing new ints.
/// Existing prefix values survive; new slots must be supplied by wire pairs,
/// otherwise their native value is unavailable in this typed input.
pub fn apply_resized_indexed_ints(
    pairs: &[HashValue<i32>],
    prior: &[i32],
    lookup: &dyn DefinitionLookup,
    limits: Limits,
) -> Result<Vec<i32>, Error> {
    check_pairs(pairs, limits)?;
    let count = lookup.count();
    if count > i32::MAX as usize {
        return Err(Error::DefinitionMapping);
    }
    let mut slots = Vec::new();
    reserve(&mut slots, count, limits)?;
    slots.extend((0..count).map(|i| prior.get(i).copied()));
    for pair in pairs {
        if let Some(i) = index(pair.definition_hash, lookup)? {
            slots[i] = Some(pair.value);
        }
    }
    let mut values = Vec::new();
    reserve(&mut values, count, limits)?;
    for (index, slot) in slots.into_iter().enumerate() {
        values.push(slot.ok_or(Error::MissingResizeValue { index })?);
    }
    Ok(values)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoodyHut {
    pub player_goodies: Vec<HashValue<i32>>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoodyHutState {
    /// Omitted from Save and unchanged by Read; never excluded as planning data.
    pub applying_goody: bool,
    pub player_goodies: Vec<i32>,
}
impl GoodyHutState {
    pub fn after_reset(lookup: &dyn DefinitionLookup, limits: Limits) -> Result<Self, Error> {
        Ok(Self {
            applying_goody: false,
            player_goodies: reset_indexed_ints(lookup, limits)?,
        })
    }
}
impl GoodyHut {
    /// Read does not call Reset; preserve the supplied applying flag and values.
    pub fn apply(
        &self,
        prior: &GoodyHutState,
        lookup: &dyn DefinitionLookup,
        limits: Limits,
    ) -> Result<GoodyHutState, Error> {
        Ok(GoodyHutState {
            applying_goody: prior.applying_goody,
            player_goodies: apply_indexed_ints(
                &self.player_goodies,
                &prior.player_goodies,
                lookup,
                limits,
            )?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bonuses {
    pub flat_bonuses: Vec<HashValue<i32>>,
}
impl Bonuses {
    /// Current original Read calls concrete Reset before the hash overlay.
    pub fn apply(&self, lookup: &dyn DefinitionLookup, limits: Limits) -> Result<Vec<i32>, Error> {
        let reset = reset_indexed_ints(lookup, limits)?;
        apply_indexed_ints(&self.flat_bonuses, &reset, lookup, limits)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CongressFavor {
    pub player: i32,
    pub lifetime: i32,
    pub entering_congress: i32,
    pub extra_per_turn: i32,
    pub suzerain_multiplier: i32,
    pub alliance_multiplier: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CongressModifiers {
    pub emergency_target: i32,
    pub emergency_member: i32,
    pub great_person: i32,
    pub bid_cost: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionEffectRefund {
    pub wire_version: i32,
    pub resolution_hash: u32,
    pub which_effect: i32,
    pub refund_percent: i32,
}
impl ResolutionEffectRefund {
    pub fn resolution_index(&self, lookup: &dyn DefinitionLookup) -> Result<i32, Error> {
        Ok(index(self.resolution_hash, lookup)?.map_or(-1, |i| i as i32))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Congress {
    /// Original saved version; only 6 and 12 have bounded schemas here.
    pub wire_version: i32,
    pub favor: CongressFavor,
    pub suzerain_favor_by_type: Vec<HashValue<i32>>,
    pub influence_token_favor_by_type: Vec<HashValue<i32>>,
    pub trade_route_favor_by_type: Vec<HashValue<i32>>,
    /// Original sequential int vector, NOT a hash overlay.
    /// None in v6: native Read self-assigns and retains the existing vector.
    /// This does not establish any constructor or initialized runtime baseline.
    pub favor_from_government_slots: Option<Vec<i32>>,
    pub trade_route_by_type_modifier: Vec<HashValue<i32>>,
    pub modifiers: CongressModifiers,
    pub resolution_effect_refunds: Vec<ResolutionEffectRefund>,
    pub wants_congress_to_meet: bool,
    pub diplo_vp_last_turn: i32,
}
impl Congress {
    /// Arrays in original order: Suzerain, InfluenceToken, TradeRoute, Modifier.
    /// Other current wire fields are direct values/replacement vectors. Read
    /// does not set omitted m_iLastTurnConsideredCongress/m_iCongressStageCompleted;
    /// their actual existing values must be retained by the owning Player.
    pub fn apply_indexed_arrays(
        &self,
        prior: [&[i32]; 4],
        lookup: &dyn DefinitionLookup,
        limits: Limits,
    ) -> Result<[Vec<i32>; 4], Error> {
        let pairs = [
            &self.suzerain_favor_by_type,
            &self.influence_token_favor_by_type,
            &self.trade_route_favor_by_type,
            &self.trade_route_by_type_modifier,
        ];
        Ok([
            apply_resized_indexed_ints(pairs[0], prior[0], lookup, limits)?,
            apply_resized_indexed_ints(pairs[1], prior[1], lookup, limits)?,
            apply_resized_indexed_ints(pairs[2], prior[2], lookup, limits)?,
            if self.wire_version == 6 {
                // Since10 array has explicit ReadIndexHashArrayVersioned zero
                // assignments for every actual definition when absent in v6.
                reset_indexed_ints(lookup, limits)?
            } else {
                apply_resized_indexed_ints(pairs[3], prior[3], lookup, limits)?
            },
        ])
    }
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl<'a> Reader<'a> {
    fn bytes(&mut self, size: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(size).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let bytes = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(bytes)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(self.u32()? as i32)
    }
    fn version(&mut self, expected: i32) -> Result<(), Error> {
        let actual = self.i32()?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(())
    }
    fn bool(&mut self) -> Result<bool, Error> {
        match self.bytes(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::InvalidBool),
        }
    }
    fn list<T>(
        &mut self,
        min_size: usize,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let count = self.u32()? as usize;
        self.items = self.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        if count > (self.data.len() - self.pos) / min_size {
            return Err(Error::Truncated);
        }
        let mut values = Vec::new();
        reserve(&mut values, count, self.limits)?;
        for _ in 0..count {
            values.push(read(self)?);
        }
        Ok(values)
    }
    fn indexed(&mut self) -> Result<Vec<HashValue<i32>>, Error> {
        self.list(8, |r| {
            Ok(HashValue {
                definition_hash: r.u32()?,
                value: r.i32()?,
            })
        })
    }
}
struct Writer {
    data: Vec<u8>,
    items: usize,
    limits: Limits,
}
impl Writer {
    fn bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if self
            .data
            .len()
            .checked_add(bytes.len())
            .ok_or(Error::ResourceLimit)?
            > self.limits.max_bytes
        {
            return Err(Error::ResourceLimit);
        }
        self.data
            .try_reserve(bytes.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.data.extend_from_slice(bytes);
        Ok(())
    }
    fn u32(&mut self, n: u32) -> Result<(), Error> {
        self.bytes(&n.to_le_bytes())
    }
    fn i32(&mut self, n: i32) -> Result<(), Error> {
        self.u32(n as u32)
    }
    fn count(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        self.u32(u32::try_from(n).map_err(|_| Error::ResourceLimit)?)
    }
    fn indexed(&mut self, pairs: &[HashValue<i32>]) -> Result<(), Error> {
        self.count(pairs.len())?;
        for pair in pairs {
            self.u32(pair.definition_hash)?;
            self.i32(pair.value)?;
        }
        Ok(())
    }
}
trait Wire: Sized {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
}
impl Wire for GoodyHut {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        Ok(Self {
            player_goodies: r.indexed()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(1)?;
        w.indexed(&self.player_goodies)
    }
}
impl Wire for Bonuses {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(4)?;
        Ok(Self {
            flat_bonuses: r.indexed()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(4)?;
        w.indexed(&self.flat_bonuses)
    }
}
impl Wire for Congress {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = r.i32()?;
        if wire_version != 6 && wire_version != 12 {
            return Err(Error::Version {
                expected: 12,
                actual: wire_version,
            });
        }
        let favor = CongressFavor {
            player: r.i32()?,
            lifetime: r.i32()?,
            entering_congress: if wire_version >= 7 { r.i32()? } else { 0 },
            extra_per_turn: r.i32()?,
            suzerain_multiplier: r.i32()?,
            alliance_multiplier: if wire_version >= 12 { r.i32()? } else { 0 },
        };
        let suzerain_favor_by_type = r.indexed()?;
        let influence_token_favor_by_type = r.indexed()?;
        let trade_route_favor_by_type = r.indexed()?;
        let favor_from_government_slots = if wire_version >= 9 {
            Some(r.list(4, |r| r.i32())?)
        } else {
            None
        };
        let trade_route_by_type_modifier = if wire_version >= 10 {
            r.indexed()?
        } else {
            Vec::new()
        };
        let modifiers = CongressModifiers {
            emergency_target: r.i32()?,
            emergency_member: r.i32()?,
            great_person: if wire_version >= 8 { r.i32()? } else { 0 },
            bid_cost: r.i32()?,
        };
        let resolution_effect_refunds = r.list(16, |r| {
            let wire_version = r.i32()?;
            if wire_version != 6 && wire_version != 12 {
                return Err(Error::Version {
                    expected: 12,
                    actual: wire_version,
                });
            }
            Ok(ResolutionEffectRefund {
                wire_version,
                resolution_hash: r.u32()?,
                which_effect: r.i32()?,
                refund_percent: r.i32()?,
            })
        })?;
        let wants_congress_to_meet = r.bool()?;
        let diplo_vp_last_turn = if wire_version >= 11 { r.i32()? } else { 0 };
        Ok(Self {
            wire_version,
            favor,
            suzerain_favor_by_type,
            influence_token_favor_by_type,
            trade_route_favor_by_type,
            favor_from_government_slots,
            trade_route_by_type_modifier,
            modifiers,
            resolution_effect_refunds,
            wants_congress_to_meet,
            diplo_vp_last_turn,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 12 {
            return Err(Error::Version {
                expected: 12,
                actual: self.wire_version,
            });
        }
        let slots = self
            .favor_from_government_slots
            .as_ref()
            .ok_or(Error::MissingGovernmentSlots)?;
        w.i32(12)?;
        let f = &self.favor;
        for value in [
            f.player,
            f.lifetime,
            f.entering_congress,
            f.extra_per_turn,
            f.suzerain_multiplier,
            f.alliance_multiplier,
        ] {
            w.i32(value)?;
        }
        w.indexed(&self.suzerain_favor_by_type)?;
        w.indexed(&self.influence_token_favor_by_type)?;
        w.indexed(&self.trade_route_favor_by_type)?;
        w.count(slots.len())?;
        for &value in slots {
            w.i32(value)?;
        }
        w.indexed(&self.trade_route_by_type_modifier)?;
        let m = &self.modifiers;
        for value in [
            m.emergency_target,
            m.emergency_member,
            m.great_person,
            m.bid_cost,
        ] {
            w.i32(value)?;
        }
        w.count(self.resolution_effect_refunds.len())?;
        for value in &self.resolution_effect_refunds {
            if value.wire_version != 12 {
                return Err(Error::Version {
                    expected: 12,
                    actual: value.wire_version,
                });
            }
            w.i32(12)?;
            w.u32(value.resolution_hash)?;
            w.i32(value.which_effect)?;
            w.i32(value.refund_percent)?;
        }
        w.bytes(&[u8::from(self.wants_congress_to_meet)])?;
        w.i32(self.diplo_vp_last_turn)
    }
}
macro_rules! codec {
    ($type:ty) => {
        impl $type {
            /// Returns the exact dynamic item charge for aggregate owner readers.
            pub fn decode_prefix_with_items(
                bytes: &[u8],
                limits: Limits,
            ) -> Result<(Self, usize, usize), Error> {
                let mut reader = Reader {
                    data: bytes,
                    pos: 0,
                    items: 0,
                    limits,
                };
                let value = Self::read(&mut reader)?;
                Ok((value, reader.pos, reader.items))
            }
            pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
                let (value, consumed, _) = Self::decode_prefix_with_items(bytes, limits)?;
                Ok((value, consumed))
            }
            pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
                let (value, consumed) = Self::decode_prefix(bytes, limits)?;
                if consumed != bytes.len() {
                    return Err(Error::Trailing);
                }
                Ok(value)
            }
            pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
                let mut writer = Writer {
                    data: Vec::new(),
                    items: 0,
                    limits,
                };
                self.write(&mut writer)?;
                Ok((writer.data, writer.items))
            }
            pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
                self.encode_with_items(limits).map(|(bytes, _)| bytes)
            }
        }
    };
}
codec!(GoodyHut);
codec!(Bonuses);
codec!(Congress);

#[cfg(test)]
mod tests {
    use super::*;
    struct Lookup(Vec<u32>);
    impl DefinitionLookup for Lookup {
        fn count(&self) -> usize {
            self.0.len()
        }
        fn find_index(&self, key: u32) -> Option<usize> {
            if (key as usize) < self.0.len() {
                Some(key as usize)
            } else {
                self.0.iter().rposition(|&h| h == key)
            }
        }
        fn type_hash(&self, index: usize) -> Option<u32> {
            self.0.get(index).copied()
        }
    }
    fn pair(hash: u32, value: i32) -> HashValue<i32> {
        HashValue {
            definition_hash: hash,
            value,
        }
    }
    fn congress() -> Congress {
        Congress {
            wire_version: 12,
            favor: CongressFavor {
                player: -1,
                lifetime: i32::MAX,
                entering_congress: i32::MIN,
                extra_per_turn: 4,
                suzerain_multiplier: 5,
                alliance_multiplier: 6,
            },
            suzerain_favor_by_type: vec![pair(90, -9), pair(90, 10), pair(999, 100)],
            influence_token_favor_by_type: vec![],
            trade_route_favor_by_type: vec![pair(70, 20)],
            favor_from_government_slots: Some(vec![1, -2, i32::MIN]),
            trade_route_by_type_modifier: vec![pair(0, 30)],
            modifiers: CongressModifiers {
                emergency_target: 7,
                emergency_member: 8,
                great_person: 9,
                bid_cost: 10,
            },
            resolution_effect_refunds: vec![ResolutionEffectRefund {
                wire_version: 12,
                resolution_hash: 1234567,
                which_effect: -1,
                refund_percent: 150,
            }],
            wants_congress_to_meet: true,
            diplo_vp_last_turn: 11,
        }
    }
    #[test]
    fn congress6_shared_reader_retains_wire_and_explicit_read_defaults() {
        // Source-only fixture with all present variable fields nonempty.
        // Omitted GovernmentSlots is an unknown existing runtime vector.
        let mut bytes = Vec::new();
        for value in [
            6i32, -7, 19, -23, 29, 1, 90, 31, 1, 90, 37, 1, 90, 41, -43, 47, -53, 1, 6, 1234567,
            -1, 150,
        ] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.push(1);
        let end = bytes.len();
        bytes.extend(123u32.to_le_bytes());
        let (value, used, items) =
            Congress::decode_prefix_with_items(&bytes, Limits::default()).unwrap();
        assert_eq!(used, end);
        assert_eq!(items, 4);
        assert_eq!(value.wire_version, 6);
        assert_eq!(value.favor.player, -7);
        assert_eq!(value.favor.lifetime, 19);
        assert_eq!(value.favor.entering_congress, 0);
        assert_eq!(value.favor.extra_per_turn, -23);
        assert_eq!(value.favor.suzerain_multiplier, 29);
        assert_eq!(value.favor.alliance_multiplier, 0);
        assert_eq!(value.favor_from_government_slots, None);
        assert!(value.trade_route_by_type_modifier.is_empty());
        assert_eq!(value.modifiers.great_person, 0);
        assert_eq!(value.diplo_vp_last_turn, 0);
        assert_eq!(value.resolution_effect_refunds[0].wire_version, 6);
        assert!(value.wants_congress_to_meet);
        let arrays = value
            .apply_indexed_arrays([&[99]; 4], &Lookup(vec![90]), Limits::default())
            .unwrap();
        assert_eq!(arrays, [vec![31], vec![37], vec![41], vec![0]]);
        assert_eq!(
            value.encode(Limits::default()),
            Err(Error::Version {
                expected: 12,
                actual: 6
            })
        );
        for length in 0..end {
            assert!(Congress::decode_prefix(&bytes[..length], Limits::default()).is_err());
        }
        assert!(Congress::decode_prefix(
            &bytes,
            Limits {
                max_bytes: end - 1,
                max_items: items
            }
        )
        .is_err());
        assert!(Congress::decode_prefix(
            &bytes,
            Limits {
                max_bytes: end,
                max_items: items - 1
            }
        )
        .is_err());
        assert_eq!(
            Congress::decode_prefix_with_items(
                &bytes,
                Limits {
                    max_bytes: end,
                    max_items: items
                }
            )
            .unwrap(),
            (value, end, items)
        );
        bytes[end - 1] = 2;
        assert_eq!(
            Congress::decode_prefix(&bytes, Limits::default()),
            Err(Error::InvalidBool)
        );
        for version in [0i32, 5, 7, 11, 13] {
            assert_eq!(
                Congress::decode_prefix(&version.to_le_bytes(), Limits::default()),
                Err(Error::Version {
                    expected: 12,
                    actual: version
                })
            );
        }
    }
    #[test]
    fn wire_roundtrips_preserve_order_duplicates_unknown_hashes_and_nested_refunds() {
        let limits = Limits::default();
        let value = congress();
        let bytes = value.encode(limits).unwrap();
        assert_eq!(Congress::decode(&bytes, limits).unwrap(), value);
        assert_eq!(
            Congress::decode_prefix(&[bytes.clone(), vec![55]].concat(), limits).unwrap(),
            (value.clone(), bytes.len())
        );
        for end in 0..bytes.len() {
            assert!(Congress::decode(&bytes[..end], limits).is_err());
        }
        assert_eq!(
            Congress::decode(&[bytes, vec![1]].concat(), limits),
            Err(Error::Trailing)
        );
        let goody = GoodyHut {
            player_goodies: vec![pair(90, -1), pair(999, 2), pair(90, 3)],
        };
        assert_eq!(
            GoodyHut::decode(&goody.encode(limits).unwrap(), limits).unwrap(),
            goody
        );
        let bonuses = Bonuses {
            flat_bonuses: goody.player_goodies,
        };
        assert_eq!(
            Bonuses::decode(&bonuses.encode(limits).unwrap(), limits).unwrap(),
            bonuses
        );
        assert_eq!(
            value.resolution_effect_refunds[0]
                .resolution_index(&Lookup(vec![1234567]))
                .unwrap(),
            0
        );
        assert_eq!(
            value.resolution_effect_refunds[0]
                .resolution_index(&Lookup(vec![]))
                .unwrap(),
            -1
        );
    }
    #[test]
    fn prefix_item_charge_composes_across_owner_components() {
        let limits = Limits::default();
        let first = GoodyHut {
            player_goodies: vec![pair(1, 2), pair(1, 3)],
        };
        let second = Bonuses {
            flat_bonuses: vec![pair(4, 5), pair(6, 7)],
        };
        let first_wire = first.encode(limits).unwrap();
        let second_wire = second.encode(limits).unwrap();
        let carrier = [first_wire.clone(), second_wire.clone(), vec![99]].concat();
        let (actual, consumed, used) = GoodyHut::decode_prefix_with_items(
            &carrier,
            Limits {
                max_items: 3,
                ..limits
            },
        )
        .unwrap();
        assert_eq!((actual, consumed, used), (first, first_wire.len(), 2));
        assert_eq!(
            Bonuses::decode_prefix_with_items(
                &carrier[consumed..],
                Limits {
                    max_items: 3 - used,
                    ..limits
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            Bonuses::decode_prefix_with_items(
                &carrier[consumed..],
                Limits {
                    max_items: 4 - used,
                    ..limits
                }
            )
            .unwrap(),
            (second, second_wire.len(), 2)
        );
        // Nested Refund records and every indexed/list entry share the count.
        let congress = congress();
        let wire = congress.encode(limits).unwrap();
        assert_eq!(
            Congress::decode_prefix_with_items(&wire, limits).unwrap().2,
            9
        );
        assert_eq!(
            congress.encode_with_items(limits).unwrap(),
            (wire.clone(), 9)
        );
        assert_eq!(
            Congress::decode_prefix_with_items(
                &wire,
                Limits {
                    max_items: 8,
                    ..limits
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            Congress::decode(&[wire, vec![99]].concat(), limits),
            Err(Error::Trailing)
        );
    }
    #[test]
    fn reset_and_overlay_have_distinct_proven_defaults_and_preserve_omitted_flag() {
        let lookup = Lookup(vec![90, 70, 90]);
        let limits = Limits::default();
        let goody = GoodyHut {
            player_goodies: vec![pair(90, 1), pair(999, -1), pair(90, -3)],
        };
        let prior = GoodyHutState {
            applying_goody: true,
            player_goodies: vec![4, 5, 6],
        };
        assert_eq!(
            goody.apply(&prior, &lookup, limits).unwrap(),
            GoodyHutState {
                applying_goody: true,
                player_goodies: vec![4, 5, -3]
            }
        );
        assert_eq!(
            GoodyHutState::after_reset(&lookup, limits).unwrap(),
            GoodyHutState {
                applying_goody: false,
                player_goodies: vec![0; 3]
            }
        );
        let bonus = Bonuses {
            flat_bonuses: goody.player_goodies,
        };
        assert_eq!(bonus.apply(&lookup, limits).unwrap(), [0, 0, -3]);
        assert_eq!(
            apply_indexed_ints(&[], &[4, 5], &lookup, limits),
            Err(Error::DestinationSize)
        );
        assert_eq!(prior.player_goodies, [4, 5, 6]);
    }
    #[test]
    fn congress_resize_preserves_prefix_and_never_guesses_new_int_bits() {
        let lookup = Lookup(vec![90, 70, 90]);
        let limits = Limits::default();
        assert_eq!(
            apply_resized_indexed_ints(&[pair(90, 7)], &[4], &lookup, limits),
            Err(Error::MissingResizeValue { index: 1 })
        );
        assert_eq!(
            apply_resized_indexed_ints(&[pair(90, 7), pair(70, 8)], &[4], &lookup, limits).unwrap(),
            [4, 8, 7]
        );
        assert_eq!(
            apply_resized_indexed_ints(&[], &[4, 5, 6, 7], &lookup, limits).unwrap(),
            [4, 5, 6]
        );
        let projected = congress()
            .apply_indexed_arrays([&[1, 2, 3]; 4], &lookup, limits)
            .unwrap();
        assert_eq!(
            projected,
            [
                vec![1, 2, 10],
                vec![1, 2, 3],
                vec![1, 20, 3],
                vec![30, 2, 3]
            ]
        );
    }
    #[test]
    fn malformed_inputs_and_resource_limits_fail_without_mutating_prior() {
        let limits = Limits::default();
        assert_eq!(
            GoodyHut::decode(&[0; 8], limits),
            Err(Error::Version {
                expected: 1,
                actual: 0
            })
        );
        let mut bytes = congress().encode(limits).unwrap();
        let bool_offset = bytes.len() - 5;
        bytes[bool_offset] = 2;
        assert_eq!(Congress::decode(&bytes, limits), Err(Error::InvalidBool));
        let small = Limits {
            max_bytes: 64,
            max_items: 1,
        };
        assert_eq!(congress().encode(small), Err(Error::ResourceLimit));
        let giant_count = [1u32.to_le_bytes(), u32::MAX.to_le_bytes()].concat();
        assert_eq!(
            GoodyHut::decode(&giant_count, limits),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            reset_indexed_ints(&Lookup(vec![1, 2]), small),
            Err(Error::ResourceLimit)
        );
    }
}
