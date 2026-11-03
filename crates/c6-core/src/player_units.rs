//! Current Linux XP2 Player.Units v22 wire transport.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::unit::Limits;

pub const VERSION: u32 = 22;
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    InvalidBool(u8),
    InvalidContainer,
    Unit(crate::unit::Error),
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Units wire: {self:?}")
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
    fn remaining(&self) -> Limits {
        Limits {
            max_bytes: self.limits.max_bytes - self.pos,
            max_items: self.limits.max_items - self.items,
        }
    }
    fn raw(&mut self, n: usize) -> Result<&[u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let b = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(b)
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn allocate<T>(&mut self, n: usize) -> Result<(), Error> {
        // Check each allocation's addressable layout; wire bytes are a separate budget.
        let bytes = n
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::ResourceLimit)?;
        if bytes > isize::MAX as usize {
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
}
struct Writer {
    data: Vec<u8>,
    items: usize,
    limits: Limits,
}
impl Writer {
    fn raw(&mut self, b: &[u8]) -> Result<(), Error> {
        let end = self
            .data
            .len()
            .checked_add(b.len())
            .ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        self.data
            .try_reserve(b.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.data.extend_from_slice(b);
        Ok(())
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
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
impl<T: Wire> Wire for Vec<T> {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)? as usize;
        r.charge(n)?;
        let needed = n.checked_mul(T::MIN).ok_or(Error::ResourceLimit)?;
        if needed > r.remaining().max_bytes {
            return Err(Error::ResourceLimit);
        }
        if needed > r.data.len().saturating_sub(r.pos) {
            return Err(Error::Truncated);
        }
        r.allocate::<T>(n)?;
        let mut v = Vec::new();
        v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            v.push(T::read(r)?)
        }
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.charge(self.len())?;
        u32::try_from(self.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        for x in self {
            x.write(w)?
        }
        Ok(())
    }
}
macro_rules! record {
    ($name:ident,  $version:expr, { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name { $(pub $field: $ty),* }
        impl Wire for $name {
            const MIN: usize = (if $version != 0 { 4 } else { 0 }) $(+ <$ty as Wire>::MIN)*;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                if $version != 0 { r.version($version)?; }
                Ok(Self { $($field: <$ty as Wire>::read(r)?),* })
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                if $version != 0 { ($version as u32).write(w)?; }
                $(self.$field.write(w)?;)*
                Ok(())
            }
        }
    };
}
use crate::unit::HashValue;
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
impl Writer {
    fn remaining(&self) -> Limits {
        Limits {
            max_bytes: self.limits.max_bytes - self.data.len(),
            max_items: self.limits.max_items - self.items,
        }
    }
}
impl Wire for bool {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        match r.raw(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            v => Err(Error::InvalidBool(v)),
        }
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.raw(&[u8::from(*self)])
    }
}
impl Wire for crate::unit::Unit {
    // Version word is an unconditional lower bound, not a native object size.
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let (v, bytes, items) =
            Self::decode_prefix_with_items(&r.data[r.pos..], r.remaining()).map_err(Error::Unit)?;
        r.charge(items)?;
        r.raw(bytes)?;
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let (bytes, items) = self.encode_with_items(w.remaining()).map_err(Error::Unit)?;
        w.charge(items)?;
        w.raw(&bytes)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Slot {
    pub index: i32,
    pub value: crate::unit::Unit,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitTable {
    pub capacity: u32,
    pub entries: Vec<Slot>,
    pub current_id_bits: u32,
    pub free_head: u32,
    pub free_tail: u32,
    /// Exactly free-count words, with no additional sequence count.
    pub free_links: Vec<u32>,
}
fn indices(capacity: u32, entries: &[Slot]) -> Result<(), Error> {
    if entries.len() > capacity as usize {
        return Err(Error::InvalidContainer);
    }
    let mut values = Vec::new();
    let bytes = entries
        .len()
        .checked_mul(std::mem::size_of::<i32>())
        .ok_or(Error::ResourceLimit)?;
    if bytes > isize::MAX as usize {
        return Err(Error::ResourceLimit);
    }
    values
        .try_reserve_exact(entries.len())
        .map_err(|_| Error::ResourceLimit)?;
    for entry in entries {
        if entry.index < 0 || entry.index as u32 >= capacity {
            return Err(Error::InvalidContainer);
        }
        values.push(entry.index);
    }
    values.sort_unstable();
    if values.windows(2).any(|v| v[0] == v[1]) {
        return Err(Error::InvalidContainer);
    }
    Ok(())
}
impl Wire for UnitTable {
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
        let needed = n.checked_mul(8).ok_or(Error::ResourceLimit)?;
        if needed > r.remaining().max_bytes {
            return Err(Error::ResourceLimit);
        }
        if needed > r.data.len().saturating_sub(r.pos) {
            return Err(Error::Truncated);
        }
        r.allocate::<Slot>(n)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(n)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            entries.push(Slot {
                index: i32::read(r)?,
                value: crate::unit::Unit::read(r)?,
            });
        }
        indices(capacity, &entries)?;
        let current_id_bits = u32::read(r)?;
        let n = u32::read(r)? as usize;
        let free_head = u32::read(r)?;
        let free_tail = u32::read(r)?;
        r.charge(n)?;
        let needed = n.checked_mul(4).ok_or(Error::ResourceLimit)?;
        if needed > r.remaining().max_bytes {
            return Err(Error::ResourceLimit);
        }
        if needed > r.data.len().saturating_sub(r.pos) {
            return Err(Error::Truncated);
        }
        r.allocate::<u32>(n)?;
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
        w.charge(self.entries.len())?;
        indices(self.capacity, &self.entries)?;
        65537u32.write(w)?;
        16u32.write(w)?;
        self.capacity.write(w)?;
        u32::try_from(self.entries.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        for entry in &self.entries {
            entry.index.write(w)?;
            entry.value.write(w)?;
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
record!(MapPair, 0, { key: u32, value: u32 });
record!(DistrictFoundingUnits, 0, { key: u32, values: Vec<MapPair> });
// These nested records write their own current Player.Units version.
// Unit and promotion enum overloads write definition hashes.
record!(RandomPromotionSelection, 22, { promotion_hash: u32, times_selected: i32 });
record!(ProposedPromotion, 22, { promotion_hash: u32, unit_id: u32 });
record!(BasePromotion, 22, { unit_hash: u32, promotion_hash: u32, count: i32 });
record!(Units, 22, {
    records: UnitTable,
    current_unit_cost: Vec<HashValue<i32>>,
    unit_production_multiplier_q8: Vec<HashValue<i32>>,
    extra_unit_copy: Vec<HashValue<i32>>,
    enable_wall_attack_whole_game_promotion_class: Vec<HashValue<i32>>,
    unit_build_disabled: Vec<HashValue<bool>>,
    no_settling_cities: bool,
    no_foreign_heal: bool,
    no_neutral_heal: bool,
    add_on_district_founding: Vec<DistrictFoundingUnits>,
    unit_free_pass_hashes: Vec<u32>,
    unit_embarkation_pass_hashes: Vec<u32>,
    extra_build_charges: i32,
    extra_build_charges_by_unit: Vec<HashValue<i32>>,
    extra_spread_charges: i32,
    embarked_movement_modifier: i32,
    upgrade_military_formation_on_city_conquest_count: i32,
    promotion_selections: Vec<RandomPromotionSelection>,
    proposed_promotions: Vec<ProposedPromotion>,
    base_promotions: Vec<BasePromotion>,
    unlimited_promotion_choice_unit_hashes: Vec<u32>,
    land_corps_unlock_civic_hash: u32,
    land_army_unlock_civic_hash: u32,
    naval_corps_unlock_civic_hash: u32,
    naval_army_unlock_civic_hash: u32,
    land_corps_unlock_tech_hash: u32,
    land_army_unlock_tech_hash: u32,
    naval_corps_unlock_tech_hash: u32,
    naval_army_unlock_tech_hash: u32,
    peak_units: u32,
    land_corps_modified_strength: i32,
    land_army_modified_strength: i32,
    naval_corps_modified_strength: i32,
    naval_army_modified_strength: i32,
    enable_wall_attack_whole_game: i32,
    rock_band_album_sales_modifier: i32,
    banned_production_yield: i32,
    favored_production_yield: i32,
    favored_production_modifier_q8: i32,
    upgrade_cost_modifier_q8: i32,
    current_unit_resource_cost: Vec<HashValue<i32>>,
    unit_fuel_demands: Vec<HashValue<i32>>,
});
impl Units {
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
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let (v, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        Ok((v, n))
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n) = Self::decode_prefix(bytes, limits)?;
        if n != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
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
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        Ok(self.encode_with_items(limits)?.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Units {
        Units {
            records: UnitTable {
                capacity: 257,
                entries: vec![],
                current_id_bits: 0xffff0000,
                free_head: 0x12345678,
                free_tail: u32::MAX,
                free_links: vec![99, 0xffff0001],
            },
            current_unit_cost: vec![
                HashValue {
                    definition_hash: u32::MAX,
                    value: i32::MIN,
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: 256,
                },
            ],
            unit_production_multiplier_q8: vec![
                HashValue {
                    definition_hash: u32::MAX,
                    value: i32::MIN,
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: 256,
                },
            ],
            extra_unit_copy: vec![
                HashValue {
                    definition_hash: u32::MAX,
                    value: i32::MIN,
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: 256,
                },
            ],
            enable_wall_attack_whole_game_promotion_class: vec![
                HashValue {
                    definition_hash: u32::MAX,
                    value: i32::MIN,
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: 256,
                },
            ],
            unit_build_disabled: vec![
                HashValue {
                    definition_hash: 42,
                    value: true,
                },
                HashValue {
                    definition_hash: 42,
                    value: false,
                },
            ],
            no_settling_cities: true,
            no_foreign_heal: true,
            no_neutral_heal: true,
            add_on_district_founding: vec![
                DistrictFoundingUnits {
                    key: u32::MAX,
                    values: vec![
                        MapPair { key: 7, value: 99 },
                        MapPair { key: 7, value: 123 },
                    ],
                },
                DistrictFoundingUnits {
                    key: u32::MAX,
                    values: vec![],
                },
            ],
            unit_free_pass_hashes: vec![u32::MAX, 0x87654321],
            unit_embarkation_pass_hashes: vec![u32::MAX, 0x87654321],
            extra_build_charges: -10,
            extra_build_charges_by_unit: vec![
                HashValue {
                    definition_hash: u32::MAX,
                    value: i32::MIN,
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: 256,
                },
            ],
            extra_spread_charges: -8,
            embarked_movement_modifier: -7,
            upgrade_military_formation_on_city_conquest_count: -6,
            promotion_selections: vec![RandomPromotionSelection {
                promotion_hash: 0xf1234567,
                times_selected: -7,
            }],
            proposed_promotions: vec![ProposedPromotion {
                promotion_hash: u32::MAX,
                unit_id: 0xffff0001,
            }],
            base_promotions: vec![BasePromotion {
                unit_hash: 0xfedcba98,
                promotion_hash: 0x87654321,
                count: i32::MIN,
            }],
            unlimited_promotion_choice_unit_hashes: vec![u32::MAX, 0x87654321],
            land_corps_unlock_civic_hash: 0xf1234567,
            land_army_unlock_civic_hash: 0xf1234567,
            naval_corps_unlock_civic_hash: 0xf1234567,
            naval_army_unlock_civic_hash: 0xf1234567,
            land_corps_unlock_tech_hash: 0xf1234567,
            land_army_unlock_tech_hash: 0xf1234567,
            naval_corps_unlock_tech_hash: 0xf1234567,
            naval_army_unlock_tech_hash: 0xf1234567,
            peak_units: 0xf1234567,
            land_corps_modified_strength: 8,
            land_army_modified_strength: 9,
            naval_corps_modified_strength: 10,
            naval_army_modified_strength: 11,
            enable_wall_attack_whole_game: 12,
            rock_band_album_sales_modifier: 13,
            banned_production_yield: 14,
            favored_production_yield: 15,
            favored_production_modifier_q8: 16,
            upgrade_cost_modifier_q8: 17,
            current_unit_resource_cost: vec![
                HashValue {
                    definition_hash: u32::MAX,
                    value: i32::MIN,
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: 256,
                },
            ],
            unit_fuel_demands: vec![
                HashValue {
                    definition_hash: u32::MAX,
                    value: i32::MIN,
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: 256,
                },
            ],
        }
    }
    fn limits() -> Limits {
        Limits {
            max_bytes: 1_000_000,
            max_items: 10_000,
        }
    }
    #[test]
    fn nonempty_tails_allocator_and_exact_caps() {
        let v = sample();
        let (bytes, items) = v.encode_with_items(limits()).unwrap();
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: items,
        };
        assert_eq!(Units::decode(&bytes, exact).unwrap(), v);
        assert_eq!(v.encode_with_items(exact).unwrap(), (bytes.clone(), items));
        for low in [
            Limits {
                max_bytes: bytes.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert!(Units::decode(&bytes, low).is_err());
            assert!(v.encode(low).is_err());
        }
        let mut joined = bytes.clone();
        joined.extend_from_slice(&[99, 88, 77]);
        assert_eq!(Units::decode(&joined, limits()), Err(Error::Trailing));
        assert_eq!(
            Units::decode_prefix_with_items(&joined, exact).unwrap(),
            (v, bytes.len(), items)
        );
        for n in 0..bytes.len() {
            assert!(Units::decode(&bytes[..n], limits()).is_err(), "prefix {n}");
        }
    }
    #[test]
    fn versions_bool_counts_and_container_refusals() {
        let v = sample();
        let bytes = v.encode(limits()).unwrap();
        for (offset, value) in [(0, 21u32), (4, 65536u32)] {
            let mut bad = bytes.clone();
            bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            assert!(matches!(
                Units::decode(&bad, limits()),
                Err(Error::Version { .. })
            ));
        }
        let mut bad = bytes.clone();
        bad[8..12].copy_from_slice(&15u32.to_le_bytes());
        assert_eq!(Units::decode(&bad, limits()), Err(Error::InvalidContainer));
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits: limits(),
        };
        VERSION.write(&mut w).unwrap();
        v.records.write(&mut w).unwrap();
        v.current_unit_cost.write(&mut w).unwrap();
        v.unit_production_multiplier_q8.write(&mut w).unwrap();
        v.extra_unit_copy.write(&mut w).unwrap();
        v.enable_wall_attack_whole_game_promotion_class
            .write(&mut w)
            .unwrap();
        let hash_bool_start = w.data.len();
        v.unit_build_disabled.write(&mut w).unwrap();
        for offset in [
            hash_bool_start + 8,
            w.data.len(),
            w.data.len() + 1,
            w.data.len() + 2,
        ] {
            let mut bad = bytes.clone();
            bad[offset] = 2;
            assert_eq!(Units::decode(&bad, limits()), Err(Error::InvalidBool(2)));
        }
        let mut r = Reader {
            data: &bytes,
            pos: hash_bool_start,
            items: 0,
            limits: limits(),
        };
        let mut bad = bytes.clone();
        bad[hash_bool_start..hash_bool_start + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        r.data = &bad;
        assert_eq!(
            Vec::<HashValue<bool>>::read(&mut r),
            Err(Error::ResourceLimit)
        );
    }
    #[test]
    fn nested_versions_are_strict_and_hashes_are_unsigned() {
        let value = sample();
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits: limits(),
        };
        value.base_promotions[0].write(&mut w).unwrap();
        assert_eq!(
            u32::from_le_bytes(w.data[4..8].try_into().unwrap()),
            0xfedcba98
        );
        w.data[0..4].copy_from_slice(&21u32.to_le_bytes());
        let mut r = Reader {
            data: &w.data,
            pos: 0,
            items: 0,
            limits: limits(),
        };
        assert_eq!(
            BasePromotion::read(&mut r),
            Err(Error::Version {
                expected: 22,
                actual: 21
            })
        );
        for data in [vec![0; 11], vec![0; 15]] {
            let mut bytes = data;
            bytes[0..4].copy_from_slice(&22u32.to_le_bytes());
            let mut r = Reader {
                data: &bytes,
                pos: 0,
                items: 0,
                limits: limits(),
            };
            assert!(BasePromotion::read(&mut r).is_err());
        }
    }
}
