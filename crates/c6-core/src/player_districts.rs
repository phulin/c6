//! Complete current Linux XP2 District v15 and Player.Districts v8 wire transport.
//! No definition remapping, live Read fixups, actual plot owner reconstruction,
//! initialized world, gameplay, actions or turns are provided by these codecs.
//! Methods and field identities: docs/linux-player-districts.md.
use crate::map::{Recycled, Slot};
use crate::unit::variants::{DecodeBudget, Limits as VariantLimits, TypedVariantMap, VariantError};
use crate::unit::{HashValue, Limits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    InvalidContainer,
    InvalidBool(u8),
    Version { expected: u32, actual: u32 },
    Variant(VariantError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Districts wire: {self:?}")
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
    fn raw(&mut self, n: usize) -> Result<&[u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let value = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(value)
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn version(&mut self, expected: u32) -> Result<(), Error> {
        let actual = u32::read(self)?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(())
    }
    fn remaining(&self) -> Limits {
        Limits {
            max_bytes: self.limits.max_bytes - self.pos,
            max_items: self.limits.max_items - self.items,
        }
    }
}
struct Writer {
    data: Vec<u8>,
    items: usize,
    limits: Limits,
}
impl Writer {
    fn raw(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let n = self
            .data
            .len()
            .checked_add(bytes.len())
            .ok_or(Error::ResourceLimit)?;
        if n > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        self.data
            .try_reserve(bytes.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.data.extend_from_slice(bytes);
        Ok(())
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn remaining(&self) -> Limits {
        Limits {
            max_bytes: self.limits.max_bytes - self.data.len(),
            max_items: self.limits.max_items - self.items,
        }
    }
}
trait Wire: Sized {
    const MIN: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
}
impl Wire for u32 {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self::from_le_bytes(r.raw(4)?.try_into().unwrap()))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.raw(&self.to_le_bytes())
    }
}
impl Wire for i32 {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(u32::read(r)? as Self)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        (*self as u32).write(w)
    }
}
impl Wire for bool {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        match r.raw(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(Error::InvalidBool(value)),
        }
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.raw(&[u8::from(*self)])
    }
}
impl<T: Wire> Wire for Vec<T> {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)? as usize;
        r.charge(n)?;
        if n > r.data.len().saturating_sub(r.pos) / T::MIN {
            return Err(Error::Truncated);
        }
        if n.checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::ResourceLimit)?
            > r.limits.max_bytes
        {
            return Err(Error::ResourceLimit);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            out.push(T::read(r)?);
        }
        Ok(out)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.charge(self.len())?;
        u32::try_from(self.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        for value in self {
            value.write(w)?;
        }
        Ok(())
    }
}
impl<A: Wire, B: Wire> Wire for (A, B) {
    const MIN: usize = A::MIN + B::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok((A::read(r)?, B::read(r)?))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.0.write(w)?;
        self.1.write(w)
    }
}
impl<T: Wire> Wire for HashValue<T> {
    const MIN: usize = 4 + T::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            definition_hash: u32::read(r)?,
            value: T::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.definition_hash.write(w)?;
        self.value.write(w)
    }
}

impl Wire for TypedVariantMap {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let left = r.remaining();
        let mut budget = DecodeBudget::new(VariantLimits {
            max_bytes: left.max_bytes,
            max_items: left.max_items,
            max_depth: 64,
        });
        let (value, n) =
            Self::decode_prefix(&r.data[r.pos..], &mut budget).map_err(Error::Variant)?;
        r.charge(budget.items_used())?;
        r.raw(n)?;
        Ok(value)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let left = w.remaining();
        let (bytes, n) = self
            .encode(VariantLimits {
                max_bytes: left.max_bytes,
                max_items: left.max_items,
                max_depth: 64,
            })
            .map_err(Error::Variant)?;
        w.charge(n)?;
        w.raw(&bytes)
    }
}
macro_rules! record {
    ($name:ident, $version:expr, { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name { $(pub $field: $ty),* }
        impl Wire for $name {
            const MIN: usize = 4 + $(<$ty as Wire>::MIN +)* 0;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                r.version($version)?; Ok(Self { $($field: <$ty>::read(r)?),* })
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                ($version as u32).write(w)?; $(self.$field.write(w)?;)* Ok(())
            }
        }
    };
}
record!(AdjacencyBonusMirror, 15, { yield_to_mirror_hash: u32, yield_to_grant_hash: u32, mirror_count: i32 });
// YieldTypes here uses its scalar enum overload: raw index, unlike Mirror's explicit hash conversion.
record!(DistrictCreateYield, 1, { district_hash: u32, yield_type: i32, amount: i32, yield_based_on_appeal: bool, must_replace_improvement: bool, improvement_hash: u32 });
record!(DistrictCreateUnit, 1, { district_hash: u32, unit_type_hash: u32 });
record!(DistrictAndBuildingsCreateUnitWithAbilityByClass, 1, { ability_hash: u32, district_hash: u32, promotion_class_hash: u32 });
record!(DistrictUnlockTech, 1, { district_hash: u32, technology_hash: u32 });
record!(DistrictUnlockCivic, 1, { district_hash: u32, civic_hash: u32 });
// Serialized fields only. The actual District owner is Plot.GetOwner and is
// deliberately absent; enclosing player/table ownership is a different value.
// Ordered hash arrays retain duplicate keys and unknown definitions verbatim.
record!(District, 15, {
    id: u32, location: (i32, i32), city_id: u32, district_hash: u32,
    garrison_damage: i32, outer_defenses_damage: i32, last_turn_damaged: i32,
    remaining_attacks: i32, cost: i32, is_complete: bool, properties: TypedVariantMap,
    yield_changes: Vec<HashValue<i32>>, base_yield_changes: Vec<HashValue<i32>>,
    yield_adjacency_modifiers: Vec<HashValue<i32>>,
    extra_regional_range: i32, extra_regional_entertainment: i32, extra_entertainment: i32,
    extra_regional_yields: Vec<HashValue<i32>>, tourism_adjacency_yield_modifier: Vec<HashValue<i32>>,
    tourism_change: i32, yield_adjacency_mirrors: Vec<AdjacencyBonusMirror>,
    great_person_point_changes_q8: Vec<HashValue<i32>>, great_person_point_multiplier: i32,
    great_person_point_adder: i32, air_slots_modifier: i32, within_one_hex_espionage_defense_bonus: i32,
    under_siege: bool, pillaged: bool,
});
impl<T: Wire> Wire for Recycled<T> {
    const MIN: usize = 32;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(65537)?;
        if u32::read(r)? != 16 {
            return Err(Error::InvalidContainer);
        }
        let capacity = u32::read(r)?;
        let n = u32::read(r)? as usize;
        if n > capacity as usize {
            return Err(Error::InvalidContainer);
        }
        r.charge(n)?;
        if n > r.data.len().saturating_sub(r.pos) / (4 + T::MIN) {
            return Err(Error::Truncated);
        }
        if n.checked_mul(std::mem::size_of::<Slot<T>>())
            .ok_or(Error::ResourceLimit)?
            > r.limits.max_bytes
        {
            return Err(Error::ResourceLimit);
        }
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(n)
            .map_err(|_| Error::ResourceLimit)?;
        let mut indexes = Vec::new();
        indexes
            .try_reserve_exact(n)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            let index = i32::read(r)?;
            if index < 0 || index as u32 >= capacity {
                return Err(Error::InvalidContainer);
            }
            indexes.push(index);
            entries.push(Slot {
                index,
                value: T::read(r)?,
            });
        }
        indexes.sort_unstable();
        if indexes.windows(2).any(|v| v[0] == v[1]) {
            return Err(Error::InvalidContainer);
        }
        let current_id_bits = u32::read(r)?;
        let n = u32::read(r)? as usize;
        let free_head = u32::read(r)?;
        let free_tail = u32::read(r)?;
        // The count precedes head/tail; links have NO second vector count.
        r.charge(n)?;
        let link_bytes = n
            .checked_mul(std::mem::size_of::<u32>())
            .ok_or(Error::ResourceLimit)?;
        if link_bytes > r.remaining().max_bytes {
            return Err(Error::ResourceLimit);
        }
        if n > r.data.len().saturating_sub(r.pos) / 4 {
            return Err(Error::Truncated);
        }
        let mut free_links = Vec::new();
        free_links
            .try_reserve_exact(n)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            free_links.push(u32::read(r)?);
        }
        Ok(Self {
            capacity,
            entries,
            current_id_bits,
            free_head,
            free_tail,
            free_links,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.entries.len() > self.capacity as usize {
            return Err(Error::InvalidContainer);
        }
        w.charge(self.entries.len())?;
        if self
            .entries
            .len()
            .checked_mul(std::mem::size_of::<i32>())
            .ok_or(Error::ResourceLimit)?
            > w.limits.max_bytes
        {
            return Err(Error::ResourceLimit);
        }
        let mut indexes = Vec::new();
        indexes
            .try_reserve_exact(self.entries.len())
            .map_err(|_| Error::ResourceLimit)?;
        65537u32.write(w)?;
        16u32.write(w)?;
        self.capacity.write(w)?;
        u32::try_from(self.entries.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        for entry in &self.entries {
            if entry.index < 0 || entry.index as u32 >= self.capacity {
                return Err(Error::InvalidContainer);
            }
            indexes.push(entry.index);
            entry.index.write(w)?;
            entry.value.write(w)?;
        }
        indexes.sort_unstable();
        if indexes.windows(2).any(|v| v[0] == v[1]) {
            return Err(Error::InvalidContainer);
        }
        self.current_id_bits.write(w)?;
        w.charge(self.free_links.len())?;
        u32::try_from(self.free_links.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        self.free_head.write(w)?;
        self.free_tail.write(w)?;
        for link in &self.free_links {
            link.write(w)?;
        }
        Ok(())
    }
}
record!(Districts, 8, {
    records: Recycled<District>, free_building_when_specialty_district_constructed: i32, envoys_non_specialty: i32,
    current_district_cost: Vec<HashValue<i32>>,
    free_building_when_specialty_district_constructed_except: Vec<HashValue<i32>>,
    specific_district_grant_envoys: Vec<HashValue<i32>>,
    district_cannot_be_built_adjacent_to_city: Vec<HashValue<i32>>,
    feature_required_for_specialty_districts: Vec<HashValue<i32>>,
    creation_yields: Vec<DistrictCreateYield>, creation_units: Vec<DistrictCreateUnit>,
    district_and_buildings_creation_units: Vec<DistrictAndBuildingsCreateUnitWithAbilityByClass>,
    unlock_techs: Vec<DistrictUnlockTech>, unlock_civics: Vec<DistrictUnlockCivic>,
});
macro_rules! codec {
    ($ty:ty) => {
        impl $ty {
            pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
                let (v, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
                if n != bytes.len() {
                    return Err(Error::Trailing);
                }
                Ok(v)
            }
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
                let v = Self::read(&mut r)?;
                Ok((v, r.pos, r.items))
            }
            pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
                Ok(self.encode_with_items(limits)?.0)
            }
            pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
                let mut w = Writer {
                    data: Vec::new(),
                    items: 0,
                    limits,
                };
                self.write(&mut w)?;
                Ok((w.data, w.items))
            }
        }
    };
}
codec!(District);
codec!(Districts);

#[cfg(test)]
mod tests {
    use super::*;
    fn word(b: &mut Vec<u8>, x: u32) {
        b.extend(x.to_le_bytes());
    }
    fn words(b: &mut Vec<u8>, xs: &[u32]) {
        for &x in xs {
            word(b, x);
        }
    }
    fn hashes(b: &mut Vec<u8>, seed: u32) {
        words(b, &[2, seed, 0x80000001, seed, 0xfffffffe]);
    }
    fn district() -> Vec<u8> {
        let mut b = Vec::new();
        words(
            &mut b,
            &[
                15, 0xf0010007, 0xfffffffd, 77, 0x50002, 0x8a32007d, 101, 102, 103, 104, 105,
            ],
        );
        b.push(1);
        // Actual supported TypedVariantMap branch: named Char8, including NUL.
        b.extend([
            1, 0, 0, 128, 1, 0, 0, 0, 7, 0, 0, 0, 2, 0, b'n', b'm', 5, 0, 0, 0, 3, 0, 0, 33, 1, 0,
            0, 0, b'x', b'y', 0,
        ]);
        for seed in [0x1230001, 0x1230002, 0x1230003] {
            hashes(&mut b, seed);
        }
        words(&mut b, &[106, 107, 108]);
        for seed in [0x1230004, 0x1230005] {
            hashes(&mut b, seed);
        }
        words(&mut b, &[109, 1, 15, 0xabcdef01, 0xabcdef02, 110]);
        hashes(&mut b, 0x1230006);
        words(&mut b, &[111, 112, 113, 114]);
        b.extend([0, 1]);
        b
    }
    fn districts() -> Vec<u8> {
        let mut b = Vec::new();
        words(&mut b, &[8, 65537, 16, 8, 2, 7]);
        b.extend(district());
        word(&mut b, 2);
        b.extend(district());
        words(&mut b, &[0xfffe0000, 3, 6, 4, 5, 4, 0xffffffff, 201, 202]);
        for seed in [0x2230001, 0x2230002, 0x2230003, 0x2230004, 0x2230005] {
            hashes(&mut b, seed);
        }
        // Five nonempty source carriers, each with its own current version.
        words(&mut b, &[1, 1, 0x2231001, 0xfffffffe, 203]);
        b.extend([1, 0]);
        word(&mut b, 0x2231002);
        words(&mut b, &[1, 1, 0x2231003, 0x2231004]);
        words(&mut b, &[1, 1, 0x2231005, 0x2231006, 0x2231007]);
        words(&mut b, &[1, 1, 0x2231008, 0x2231009]);
        words(&mut b, &[1, 1, 0x2231010, 0x2231011]);
        b
    }
    #[test]
    fn nonempty_source_carriers_preserve_all_fields_and_allocator_order() {
        let b = district();
        let (v, n, items) = District::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!(n, b.len());
        assert_eq!(v.location, (-3, 77));
        assert_eq!(v.id, 0xf0010007);
        assert_eq!(
            (
                v.garrison_damage,
                v.outer_defenses_damage,
                v.last_turn_damaged,
                v.remaining_attacks,
                v.cost
            ),
            (101, 102, 103, 104, 105)
        );
        assert!(v.is_complete);
        assert_eq!(v.properties.entries[0].name.as_deref(), Some(&b"nm"[..]));
        assert_eq!(
            (
                v.extra_regional_range,
                v.extra_regional_entertainment,
                v.extra_entertainment,
                v.tourism_change
            ),
            (106, 107, 108, 109)
        );
        assert_eq!(
            v.yield_adjacency_mirrors[0].yield_to_mirror_hash,
            0xabcdef01
        );
        assert_eq!(v.great_person_point_changes_q8[0].value, i32::MIN + 1);
        assert_eq!(
            (
                v.great_person_point_multiplier,
                v.great_person_point_adder,
                v.air_slots_modifier,
                v.within_one_hex_espionage_defense_bonus
            ),
            (111, 112, 113, 114)
        );
        assert!(!v.under_siege && v.pillaged);
        assert_eq!(v.encode_with_items(Limits::default()).unwrap(), (b, items));
        let b = districts();
        let (v, n, items) = Districts::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!(n, b.len());
        assert_eq!(
            v.records
                .entries
                .iter()
                .map(|x| x.index)
                .collect::<Vec<_>>(),
            [7, 2]
        );
        assert_eq!(v.records.free_links, [5, 4, u32::MAX]);
        assert_eq!(
            (
                v.records.current_id_bits,
                v.records.free_head,
                v.records.free_tail
            ),
            (0xfffe0000, 6, 4)
        );
        assert_eq!(v.creation_yields[0].yield_type, -2);
        assert_eq!(v.unlock_techs[0].technology_hash, 0x2231009);
        assert_eq!(v.encode_with_items(Limits::default()).unwrap(), (b, items));
    }
    #[test]
    fn free_links_preflight_remaining_byte_budget_before_allocation_or_reading() {
        let links = 4096u32;
        let mut bytes = Vec::new();
        words(
            &mut bytes,
            &[65537, 16, links, 0, 0x12340000, links, 0, links - 1],
        );
        for link in 0..links {
            word(&mut bytes, link);
        }
        // The complete source is available and the item budget allows all links.
        // A cap allowing only a prefix of the links must fail at their start;
        // incremental scalar enforcement would already consume some links.
        for max_bytes in [32, 36, 8192] {
            let mut reader = Reader {
                data: &bytes,
                pos: 0,
                items: 0,
                limits: Limits {
                    max_bytes,
                    max_items: links as usize,
                },
            };
            assert_eq!(
                Recycled::<District>::read(&mut reader),
                Err(Error::ResourceLimit)
            );
            assert_eq!(reader.pos, 32);
        }
        let mut reader = Reader {
            data: &bytes,
            pos: 0,
            items: 0,
            limits: Limits {
                max_bytes: bytes.len(),
                max_items: links as usize,
            },
        };
        let value = Recycled::<District>::read(&mut reader).unwrap();
        assert_eq!(value.free_links.len(), links as usize);
        assert_eq!(reader.pos, bytes.len());
    }

    #[test]
    fn all_truncations_versions_bools_trailing_and_exact_shared_limits() {
        for b in [district(), districts()] {
            let envelope = b[0] == 8;
            for n in 0..b.len() {
                if envelope {
                    assert!(
                        Districts::decode(&b[..n], Limits::default()).is_err(),
                        "cut {n}"
                    );
                } else {
                    assert!(
                        District::decode(&b[..n], Limits::default()).is_err(),
                        "cut {n}"
                    );
                }
            }
            let mut trailing = b.clone();
            trailing.push(9);
            if envelope {
                assert_eq!(
                    Districts::decode(&trailing, Limits::default()),
                    Err(Error::Trailing)
                );
            } else {
                assert_eq!(
                    District::decode(&trailing, Limits::default()),
                    Err(Error::Trailing)
                );
            }
            let (items, reencode) = if envelope {
                let (v, _, items) =
                    Districts::decode_prefix_with_items(&b, Limits::default()).unwrap();
                (
                    items,
                    v.encode_with_items(Limits {
                        max_bytes: b.len(),
                        max_items: items,
                    })
                    .unwrap(),
                )
            } else {
                let (v, _, items) =
                    District::decode_prefix_with_items(&b, Limits::default()).unwrap();
                (
                    items,
                    v.encode_with_items(Limits {
                        max_bytes: b.len(),
                        max_items: items,
                    })
                    .unwrap(),
                )
            };
            assert_eq!(reencode, (b.clone(), items));
            for limit in [
                Limits {
                    max_bytes: b.len() - 1,
                    max_items: items,
                },
                Limits {
                    max_bytes: b.len(),
                    max_items: items - 1,
                },
            ] {
                if envelope {
                    assert!(Districts::decode(&b, limit).is_err());
                    assert!(Districts::decode(&b, Limits::default())
                        .unwrap()
                        .encode(limit)
                        .is_err());
                } else {
                    assert!(District::decode(&b, limit).is_err());
                    assert!(District::decode(&b, Limits::default())
                        .unwrap()
                        .encode(limit)
                        .is_err());
                }
            }
        }
        let original = district();
        for offset in [0, original.len() - 54] {
            // District and Mirror versions.
            let mut b = original.clone();
            b[offset..offset + 4].copy_from_slice(&99u32.to_le_bytes());
            assert!(District::decode(&b, Limits::default()).is_err());
        }
        for offset in [44, original.len() - 2, original.len() - 1] {
            let mut b = original.clone();
            b[offset] = 2;
            assert_eq!(
                District::decode(&b, Limits::default()),
                Err(Error::InvalidBool(2))
            );
        }
        let envelope = districts();
        for back in [90, 64, 48, 28, 12] {
            let mut b = envelope.clone();
            let offset = b.len() - back;
            b[offset..offset + 4].copy_from_slice(&99u32.to_le_bytes());
            assert!(matches!(
                Districts::decode(&b, Limits::default()),
                Err(Error::Version {
                    expected: 1,
                    actual: 99
                })
            ));
        }
        for back in [74, 73] {
            let mut b = envelope.clone();
            let offset = b.len() - back;
            b[offset] = 2;
            assert_eq!(
                Districts::decode(&b, Limits::default()),
                Err(Error::InvalidBool(2))
            );
        }
        let mut b = districts();
        b[4..8].copy_from_slice(&1u32.to_le_bytes());
        assert!(matches!(
            Districts::decode(&b, Limits::default()),
            Err(Error::Version { .. })
        ));
        let mut b = districts();
        b[8..12].copy_from_slice(&15u32.to_le_bytes());
        assert_eq!(
            Districts::decode(&b, Limits::default()),
            Err(Error::InvalidContainer)
        );
        let mut b = districts();
        let second_slot = 24 + original.len();
        b[second_slot..second_slot + 4].copy_from_slice(&7u32.to_le_bytes());
        assert_eq!(
            Districts::decode(&b, Limits::default()),
            Err(Error::InvalidContainer)
        );
        let mut b = district();
        b[49..53].copy_from_slice(&0x7fffffffu32.to_le_bytes());
        assert!(District::decode(&b, Limits::default()).is_err());
    }
}
