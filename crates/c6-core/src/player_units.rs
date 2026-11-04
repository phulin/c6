//! Linux XP2 Player.Units v16/v22 wire transport.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::unit::Limits;

pub const VERSION: u32 = 22;
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    UnsupportedLegacyWrite,
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
    unit_start: Option<usize>,
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
            let index = i32::read(r)?;
            r.unit_start = Some(r.pos);
            let value = crate::unit::Unit::read(r)?;
            r.unit_start = None;
            entries.push(Slot { index, value });
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
// These two readers consume their own tag and then both fields unconditionally.
// Admit the observed legacy16 and current22 profiles without borrowing the outer tag.
macro_rules! promotion_record {
    ($name:ident, $field:ident: $ty:ty) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name {
            pub wire_version: u32,
            pub promotion_hash: u32,
            pub $field: $ty,
        }
        impl Wire for $name {
            const MIN: usize = 12;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                let wire_version = u32::read(r)?;
                if wire_version != 16 && wire_version != VERSION {
                    return Err(Error::Version {
                        expected: VERSION,
                        actual: wire_version,
                    });
                }
                Ok(Self {
                    wire_version,
                    promotion_hash: u32::read(r)?,
                    $field: <$ty>::read(r)?,
                })
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                if self.wire_version != VERSION {
                    return Err(Error::UnsupportedLegacyWrite);
                }
                self.wire_version.write(w)?;
                self.promotion_hash.write(w)?;
                self.$field.write(w)
            }
        }
    };
}
promotion_record!(RandomPromotionSelection, times_selected: i32);
promotion_record!(ProposedPromotion, unit_id: u32);
record!(BasePromotion, 22, { unit_hash: u32, promotion_hash: u32, count: i32 });
// Unlike the other sequence counts, native free-pass count is signed i32:
// writer 0x2ba1bae; reader 0x2ba2136, sign extension 0x2ba214a.
// Negative carriers are explicitly unsupported before any allocation/charge.
fn read_free_pass_hashes(r: &mut Reader<'_>) -> Result<Vec<u32>, Error> {
    let n = usize::try_from(i32::read(r)?).map_err(|_| Error::InvalidContainer)?;
    r.charge(n)?;
    let needed = n.checked_mul(4).ok_or(Error::ResourceLimit)?;
    if needed > r.remaining().max_bytes {
        return Err(Error::ResourceLimit);
    }
    if needed > r.data.len().saturating_sub(r.pos) {
        return Err(Error::Truncated);
    }
    r.allocate::<u32>(n)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(n)
        .map_err(|_| Error::ResourceLimit)?;
    for _ in 0..n {
        values.push(u32::read(r)?);
    }
    Ok(values)
}
fn free_pass_count(n: usize) -> Result<i32, Error> {
    i32::try_from(n).map_err(|_| Error::InvalidContainer)
}
fn write_free_pass_hashes(values: &[u32], w: &mut Writer) -> Result<(), Error> {
    let n = free_pass_count(values.len())?;
    w.charge(values.len())?;
    n.write(w)?;
    for value in values {
        value.write(w)?;
    }
    Ok(())
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Units {
    pub wire_version: u32,
    /// At v16 these are unkeyed native-vector values, not hash/index pairs.
    /// No actual promotion-class registry ordering is inferred here.
    pub legacy_wall_attack_promotion_class_values: Option<Vec<i32>>,
    pub records: UnitTable,
    pub current_unit_cost: Vec<HashValue<i32>>,
    pub unit_production_multiplier_q8: Vec<HashValue<i32>>,
    pub extra_unit_copy: Vec<HashValue<i32>>,
    pub enable_wall_attack_whole_game_promotion_class: Vec<HashValue<i32>>,
    pub unit_build_disabled: Vec<HashValue<bool>>,
    pub no_settling_cities: bool,
    pub no_foreign_heal: bool,
    pub no_neutral_heal: bool,
    pub add_on_district_founding: Vec<DistrictFoundingUnits>,
    pub unit_free_pass_hashes: Vec<u32>,
    pub unit_embarkation_pass_hashes: Vec<u32>,
    pub extra_build_charges: i32,
    pub extra_build_charges_by_unit: Vec<HashValue<i32>>,
    pub extra_spread_charges: i32,
    pub embarked_movement_modifier: i32,
    pub upgrade_military_formation_on_city_conquest_count: i32,
    pub promotion_selections: Vec<RandomPromotionSelection>,
    pub proposed_promotions: Vec<ProposedPromotion>,
    pub base_promotions: Vec<BasePromotion>,
    pub unlimited_promotion_choice_unit_hashes: Vec<u32>,
    pub land_corps_unlock_civic_hash: u32,
    pub land_army_unlock_civic_hash: u32,
    pub naval_corps_unlock_civic_hash: u32,
    pub naval_army_unlock_civic_hash: u32,
    pub land_corps_unlock_tech_hash: u32,
    pub land_army_unlock_tech_hash: u32,
    pub naval_corps_unlock_tech_hash: u32,
    pub naval_army_unlock_tech_hash: u32,
    pub peak_units: u32,
    pub land_corps_modified_strength: i32,
    pub land_army_modified_strength: i32,
    pub naval_corps_modified_strength: i32,
    pub naval_army_modified_strength: i32,
    pub enable_wall_attack_whole_game: i32,
    pub rock_band_album_sales_modifier: i32,
    pub banned_production_yield: i32,
    pub favored_production_yield: i32,
    pub favored_production_modifier_q8: i32,
    pub upgrade_cost_modifier_q8: i32,
    pub current_unit_resource_cost: Vec<HashValue<i32>>,
    pub unit_fuel_demands: Vec<HashValue<i32>>,
}
impl Wire for Units {
    const MIN: usize = 4 + <UnitTable as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if wire_version != 16 && wire_version != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual: wire_version,
            });
        }
        let records = <UnitTable as Wire>::read(r)?;
        let current_unit_cost = <Vec<HashValue<i32>> as Wire>::read(r)?;
        let unit_production_multiplier_q8 = <Vec<HashValue<i32>> as Wire>::read(r)?;
        let extra_unit_copy = <Vec<HashValue<i32>> as Wire>::read(r)?;
        let legacy_wall_attack_promotion_class_values = if wire_version == 16 {
            Some(Vec::<i32>::read(r)?)
        } else {
            None
        };
        let enable_wall_attack_whole_game_promotion_class = if wire_version == 16 {
            Vec::new()
        } else {
            Vec::<HashValue<i32>>::read(r)?
        };
        Ok(Self {
            wire_version,
            legacy_wall_attack_promotion_class_values,
            records,
            current_unit_cost,
            unit_production_multiplier_q8,
            extra_unit_copy,
            enable_wall_attack_whole_game_promotion_class,
            unit_build_disabled: <Vec<HashValue<bool>> as Wire>::read(r)?,
            no_settling_cities: <bool as Wire>::read(r)?,
            no_foreign_heal: <bool as Wire>::read(r)?,
            no_neutral_heal: <bool as Wire>::read(r)?,
            add_on_district_founding: <Vec<DistrictFoundingUnits> as Wire>::read(r)?,
            unit_free_pass_hashes: read_free_pass_hashes(r)?,
            unit_embarkation_pass_hashes: if wire_version == 16 {
                Vec::new()
            } else {
                <Vec<u32> as Wire>::read(r)?
            },
            extra_build_charges: <i32 as Wire>::read(r)?,
            extra_build_charges_by_unit: if wire_version == 16 {
                Vec::new()
            } else {
                <Vec<HashValue<i32>> as Wire>::read(r)?
            },
            extra_spread_charges: <i32 as Wire>::read(r)?,
            embarked_movement_modifier: <i32 as Wire>::read(r)?,
            upgrade_military_formation_on_city_conquest_count: <i32 as Wire>::read(r)?,
            promotion_selections: <Vec<RandomPromotionSelection> as Wire>::read(r)?,
            proposed_promotions: <Vec<ProposedPromotion> as Wire>::read(r)?,
            base_promotions: if wire_version == 16 {
                Vec::new()
            } else {
                <Vec<BasePromotion> as Wire>::read(r)?
            },
            unlimited_promotion_choice_unit_hashes: <Vec<u32> as Wire>::read(r)?,
            land_corps_unlock_civic_hash: <u32 as Wire>::read(r)?,
            land_army_unlock_civic_hash: <u32 as Wire>::read(r)?,
            naval_corps_unlock_civic_hash: <u32 as Wire>::read(r)?,
            naval_army_unlock_civic_hash: <u32 as Wire>::read(r)?,
            land_corps_unlock_tech_hash: <u32 as Wire>::read(r)?,
            land_army_unlock_tech_hash: <u32 as Wire>::read(r)?,
            naval_corps_unlock_tech_hash: <u32 as Wire>::read(r)?,
            naval_army_unlock_tech_hash: <u32 as Wire>::read(r)?,
            peak_units: <u32 as Wire>::read(r)?,
            land_corps_modified_strength: <i32 as Wire>::read(r)?,
            land_army_modified_strength: <i32 as Wire>::read(r)?,
            naval_corps_modified_strength: <i32 as Wire>::read(r)?,
            naval_army_modified_strength: <i32 as Wire>::read(r)?,
            enable_wall_attack_whole_game: <i32 as Wire>::read(r)?,
            rock_band_album_sales_modifier: if wire_version == 16 {
                0
            } else {
                <i32 as Wire>::read(r)?
            },
            banned_production_yield: <i32 as Wire>::read(r)?,
            favored_production_yield: <i32 as Wire>::read(r)?,
            favored_production_modifier_q8: <i32 as Wire>::read(r)?,
            upgrade_cost_modifier_q8: if wire_version == 16 {
                0
            } else {
                <i32 as Wire>::read(r)?
            },
            current_unit_resource_cost: <Vec<HashValue<i32>> as Wire>::read(r)?,
            unit_fuel_demands: if wire_version == 16 {
                Vec::new()
            } else {
                <Vec<HashValue<i32>> as Wire>::read(r)?
            },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        if self.legacy_wall_attack_promotion_class_values.is_some() {
            return Err(Error::InvalidContainer);
        }
        22u32.write(w)?;
        self.records.write(w)?;
        self.current_unit_cost.write(w)?;
        self.unit_production_multiplier_q8.write(w)?;
        self.extra_unit_copy.write(w)?;
        self.enable_wall_attack_whole_game_promotion_class
            .write(w)?;
        self.unit_build_disabled.write(w)?;
        self.no_settling_cities.write(w)?;
        self.no_foreign_heal.write(w)?;
        self.no_neutral_heal.write(w)?;
        self.add_on_district_founding.write(w)?;
        write_free_pass_hashes(&self.unit_free_pass_hashes, w)?;
        self.unit_embarkation_pass_hashes.write(w)?;
        self.extra_build_charges.write(w)?;
        self.extra_build_charges_by_unit.write(w)?;
        self.extra_spread_charges.write(w)?;
        self.embarked_movement_modifier.write(w)?;
        self.upgrade_military_formation_on_city_conquest_count
            .write(w)?;
        self.promotion_selections.write(w)?;
        self.proposed_promotions.write(w)?;
        self.base_promotions.write(w)?;
        self.unlimited_promotion_choice_unit_hashes.write(w)?;
        self.land_corps_unlock_civic_hash.write(w)?;
        self.land_army_unlock_civic_hash.write(w)?;
        self.naval_corps_unlock_civic_hash.write(w)?;
        self.naval_army_unlock_civic_hash.write(w)?;
        self.land_corps_unlock_tech_hash.write(w)?;
        self.land_army_unlock_tech_hash.write(w)?;
        self.naval_corps_unlock_tech_hash.write(w)?;
        self.naval_army_unlock_tech_hash.write(w)?;
        self.peak_units.write(w)?;
        self.land_corps_modified_strength.write(w)?;
        self.land_army_modified_strength.write(w)?;
        self.naval_corps_modified_strength.write(w)?;
        self.naval_army_modified_strength.write(w)?;
        self.enable_wall_attack_whole_game.write(w)?;
        self.rock_band_album_sales_modifier.write(w)?;
        self.banned_production_yield.write(w)?;
        self.favored_production_yield.write(w)?;
        self.favored_production_modifier_q8.write(w)?;
        self.upgrade_cost_modifier_q8.write(w)?;
        self.current_unit_resource_cost.write(w)?;
        self.unit_fuel_demands.write(w)?;
        Ok(())
    }
}
/// Cursor at the failed read, or exact Unit start when the delegated Unit
/// decoder fails. The nested error does not imply its own tag was the Unit tag.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Boundary {
    /// Parent aggregate item charge before the refused delegated Unit starts.
    pub items_used: usize,
    pub component: &'static str,
    pub consumed: usize,
    pub error: Error,
}
/// Unapplied native Read migration, conditional on an actual registry lookup
/// and an in-range destination index. The old array value is a separate input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuildChargeMigration {
    pub unit_type_hash: u32,
    pub delta: i32,
}
impl Units {
    /// v16 omits embarkationPass and basePromotions: native self-copy retains
    /// prior values. Their empty DTO vectors represent no serialized elements.
    pub fn omitted_lists_retain_prior_value(&self) -> bool {
        self.wire_version == 16
    }
    /// v16 omits fuel demand: native Read fills int0 at actual Resource indexes.
    /// Registry count/order and other logical slots are not inferred.
    pub fn fuel_demands_zero_at_actual_resource_indexes(&self) -> bool {
        self.wire_version == 16
    }
    /// For v16, a positive scalar may update the prior per-Unit array at Builder.
    /// Native ChangeExtraBuildCharges uses wrapping add and clamps a negative
    /// signed result to0; invalid/missing actual indexes leave it unchanged.
    pub fn extra_build_charges_read_migration(&self) -> Option<BuildChargeMigration> {
        (self.wire_version == 16 && self.extra_build_charges > 0).then_some(BuildChargeMigration {
            unit_type_hash: 0x11d57532,
            delta: self.extra_build_charges,
        })
    }
    pub fn decode_prefix_with_boundary(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Boundary> {
        let mut r = Reader {
            data: bytes,
            pos: 0,
            items: 0,
            limits,
            unit_start: None,
        };
        match Self::read(&mut r) {
            Ok(value) => Ok((value, r.pos, r.items)),
            Err(error) => Err(Boundary {
                items_used: r.items,
                component: if r.unit_start.is_some() {
                    "Unit"
                } else {
                    "Units"
                },
                consumed: r.unit_start.unwrap_or(r.pos),
                error,
            }),
        }
    }
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        Self::decode_prefix_with_boundary(bytes, limits).map_err(|boundary| boundary.error)
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
            wire_version: VERSION,
            legacy_wall_attack_promotion_class_values: None,
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
                wire_version: 22,
                promotion_hash: 0xf1234567,
                times_selected: -7,
            }],
            proposed_promotions: vec![ProposedPromotion {
                wire_version: 22,
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
            unit_start: None,
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
            unit_start: None,
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
                unit_start: None,
                data: &bytes,
                pos: 0,
                items: 0,
                limits: limits(),
            };
            assert!(BasePromotion::read(&mut r).is_err());
        }
    }
    #[test]
    fn signed_free_pass_count_refuses_negative_before_allocation() {
        let huge = Limits {
            max_bytes: usize::MAX,
            max_items: usize::MAX,
        };
        for count in [0x80000000u32, u32::MAX] {
            let data = count.to_le_bytes();
            let mut r = Reader {
                unit_start: None,
                data: &data,
                pos: 0,
                items: 0,
                limits: huge,
            };
            assert_eq!(read_free_pass_hashes(&mut r), Err(Error::InvalidContainer));
            assert_eq!((r.pos, r.items), (4, 0));
        }
        assert_eq!(free_pass_count(i32::MAX as usize), Ok(i32::MAX));
        assert_eq!(
            free_pass_count(i32::MAX as usize + 1),
            Err(Error::InvalidContainer)
        );
        assert_eq!(free_pass_count(usize::MAX), Err(Error::InvalidContainer));
        let values = vec![u32::MAX, 0x87654321];
        let exact = Limits {
            max_bytes: 12,
            max_items: 2,
        };
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits: exact,
        };
        write_free_pass_hashes(&values, &mut w).unwrap();
        assert_eq!(&w.data[..4], &2i32.to_le_bytes());
        let mut r = Reader {
            unit_start: None,
            data: &w.data,
            pos: 0,
            items: 0,
            limits: exact,
        };
        assert_eq!(read_free_pass_hashes(&mut r).unwrap(), values);
        assert_eq!((r.pos, r.items), (12, 2));
        for low in [
            Limits {
                max_bytes: 11,
                ..exact
            },
            Limits {
                max_items: 1,
                ..exact
            },
        ] {
            let mut r = Reader {
                unit_start: None,
                data: &w.data,
                pos: 0,
                items: 0,
                limits: low,
            };
            assert_eq!(read_free_pass_hashes(&mut r), Err(Error::ResourceLimit));
            let mut short = Writer {
                data: Vec::new(),
                items: 0,
                limits: low,
            };
            assert_eq!(
                write_free_pass_hashes(&values, &mut short),
                Err(Error::ResourceLimit)
            );
        }
        // Verify that the complete envelope dispatches this field through the
        // signed adapter, rather than the ordinary u32-count Vec grammar.
        let v = sample();
        let mut prefix = Writer {
            data: Vec::new(),
            items: 0,
            limits: limits(),
        };
        22u32.write(&mut prefix).unwrap();
        v.records.write(&mut prefix).unwrap();
        v.current_unit_cost.write(&mut prefix).unwrap();
        v.unit_production_multiplier_q8.write(&mut prefix).unwrap();
        v.extra_unit_copy.write(&mut prefix).unwrap();
        v.enable_wall_attack_whole_game_promotion_class
            .write(&mut prefix)
            .unwrap();
        v.unit_build_disabled.write(&mut prefix).unwrap();
        v.no_settling_cities.write(&mut prefix).unwrap();
        v.no_foreign_heal.write(&mut prefix).unwrap();
        v.no_neutral_heal.write(&mut prefix).unwrap();
        v.add_on_district_founding.write(&mut prefix).unwrap();
        prefix.data.extend_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Units::decode_prefix(&prefix.data, huge),
            Err(Error::InvalidContainer)
        );
    }
    fn legacy16_fixture() -> (Vec<u8>, Units) {
        let mut value = sample();
        value.records.entries.clear();
        value.extra_build_charges = 7;
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits: limits(),
        };
        16u32.write(&mut w).unwrap();
        value.records.write(&mut w).unwrap();
        value.current_unit_cost.write(&mut w).unwrap();
        value.unit_production_multiplier_q8.write(&mut w).unwrap();
        value.extra_unit_copy.write(&mut w).unwrap();
        vec![7i32, -3, i32::MIN].write(&mut w).unwrap();
        value.unit_build_disabled.write(&mut w).unwrap();
        value.no_settling_cities.write(&mut w).unwrap();
        value.no_foreign_heal.write(&mut w).unwrap();
        value.no_neutral_heal.write(&mut w).unwrap();
        value.add_on_district_founding.write(&mut w).unwrap();
        write_free_pass_hashes(&value.unit_free_pass_hashes, &mut w).unwrap();
        value.extra_build_charges.write(&mut w).unwrap();
        value.extra_spread_charges.write(&mut w).unwrap();
        value.embarked_movement_modifier.write(&mut w).unwrap();
        value
            .upgrade_military_formation_on_city_conquest_count
            .write(&mut w)
            .unwrap();
        value.promotion_selections.write(&mut w).unwrap();
        value.proposed_promotions.write(&mut w).unwrap();
        value
            .unlimited_promotion_choice_unit_hashes
            .write(&mut w)
            .unwrap();
        value.land_corps_unlock_civic_hash.write(&mut w).unwrap();
        value.land_army_unlock_civic_hash.write(&mut w).unwrap();
        value.naval_corps_unlock_civic_hash.write(&mut w).unwrap();
        value.naval_army_unlock_civic_hash.write(&mut w).unwrap();
        value.land_corps_unlock_tech_hash.write(&mut w).unwrap();
        value.land_army_unlock_tech_hash.write(&mut w).unwrap();
        value.naval_corps_unlock_tech_hash.write(&mut w).unwrap();
        value.naval_army_unlock_tech_hash.write(&mut w).unwrap();
        value.peak_units.write(&mut w).unwrap();
        value.land_corps_modified_strength.write(&mut w).unwrap();
        value.land_army_modified_strength.write(&mut w).unwrap();
        value.naval_corps_modified_strength.write(&mut w).unwrap();
        value.naval_army_modified_strength.write(&mut w).unwrap();
        value.enable_wall_attack_whole_game.write(&mut w).unwrap();
        value.banned_production_yield.write(&mut w).unwrap();
        value.favored_production_yield.write(&mut w).unwrap();
        value.favored_production_modifier_q8.write(&mut w).unwrap();
        value.current_unit_resource_cost.write(&mut w).unwrap();
        value.wire_version = 16;
        value.legacy_wall_attack_promotion_class_values = Some(vec![7, -3, i32::MIN]);
        value.enable_wall_attack_whole_game_promotion_class.clear();
        value.unit_embarkation_pass_hashes.clear();
        value.extra_build_charges_by_unit.clear();
        value.base_promotions.clear();
        value.rock_band_album_sales_modifier = 0;
        value.upgrade_cost_modifier_q8 = 0;
        value.unit_fuel_demands.clear();
        (w.data, value)
    }
    #[test]
    fn legacy16_raw_vector_and_unapplied_read_effects() {
        let (b, expected) = legacy16_fixture();
        let (mut value, used, items) = Units::decode_prefix_with_items(&b, limits()).unwrap();
        assert_eq!(value, expected);
        assert_eq!(used, b.len());
        assert!(value.omitted_lists_retain_prior_value());
        assert!(value.fuel_demands_zero_at_actual_resource_indexes());
        assert_eq!(
            value.extra_build_charges_read_migration(),
            Some(BuildChargeMigration {
                unit_type_hash: 0x11d57532,
                delta: 7
            })
        );
        for delta in [-1, 0] {
            value.extra_build_charges = delta;
            assert_eq!(value.extra_build_charges_read_migration(), None);
        }
        assert_eq!(value.encode(limits()), Err(Error::UnsupportedLegacyWrite));
        assert!(Units::decode_prefix_with_items(
            &b,
            Limits {
                max_bytes: b.len(),
                max_items: items
            }
        )
        .is_ok());
        assert_eq!(
            Units::decode_prefix_with_items(
                &b,
                Limits {
                    max_bytes: b.len() - 1,
                    max_items: items
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            Units::decode_prefix_with_items(
                &b,
                Limits {
                    max_bytes: b.len(),
                    max_items: items - 1
                }
            ),
            Err(Error::ResourceLimit)
        );
        for end in 0..b.len() {
            assert!(Units::decode(&b[..end], limits()).is_err(), "prefix {end}");
        }
        let mut unsupported = b.clone();
        unsupported[..4].copy_from_slice(&17u32.to_le_bytes());
        assert_eq!(
            Units::decode(&unsupported, limits()),
            Err(Error::Version {
                expected: 22,
                actual: 17
            })
        );
        let mut trailing = b;
        trailing.push(0);
        assert_eq!(Units::decode(&trailing, limits()), Err(Error::Trailing));
        let current = sample();
        assert!(!current.omitted_lists_retain_prior_value());
        assert!(!current.fuel_demands_zero_at_actual_resource_indexes());
        assert_eq!(current.extra_build_charges_read_migration(), None);
    }
    #[test]
    fn independent_unit_failure_reports_exact_child_start() {
        let mut b = Vec::new();
        for value in [16u32, 65537, 16, 1, 1, 0, 0] {
            b.extend_from_slice(&value.to_le_bytes());
        }
        let boundary = Units::decode_prefix_with_boundary(&b, limits()).unwrap_err();
        assert_eq!((boundary.component, boundary.consumed), ("Unit", 24));
        assert_eq!(boundary.items_used, 1);
        assert!(matches!(
            boundary.error,
            Error::Unit(crate::unit::Error::Version { actual: 0, .. })
        ));
        assert_eq!(
            Units::decode_prefix_with_items(&b, limits()),
            Err(boundary.error)
        );
        b[..4].copy_from_slice(&17u32.to_le_bytes());
        let boundary = Units::decode_prefix_with_boundary(&b, limits()).unwrap_err();
        assert_eq!((boundary.component, boundary.consumed), ("Units", 4));
    }
    #[test]
    fn legacy16_promotion_records_keep_independent_tags_and_identifiers() {
        let value = sample();
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits: Limits::default(),
        };
        value.promotion_selections.write(&mut w).unwrap();
        value.proposed_promotions.write(&mut w).unwrap();
        assert_eq!(w.data.len(), 32);
        w.data[4..8].copy_from_slice(&16u32.to_le_bytes());
        w.data[20..24].copy_from_slice(&16u32.to_le_bytes());
        let read = |data: &[u8], budget: usize| {
            let mut r = Reader {
                data,
                pos: 0,
                items: 0,
                limits: Limits {
                    max_bytes: 32,
                    max_items: budget,
                },
                unit_start: None,
            };
            let selections = Vec::<RandomPromotionSelection>::read(&mut r)?;
            let proposed = Vec::<ProposedPromotion>::read(&mut r)?;
            Ok::<_, Error>((selections, proposed, r.pos, r.items))
        };
        let (selections, proposed, n, items) = read(&w.data, 2).unwrap();
        assert_eq!((n, items), (32, 2));
        assert_eq!(selections[0].wire_version, 16);
        assert_eq!(selections[0].promotion_hash, 0xf1234567);
        assert_eq!(selections[0].times_selected, -7);
        assert_eq!(proposed[0].wire_version, 16);
        assert_eq!(proposed[0].promotion_hash, u32::MAX);
        assert_eq!(proposed[0].unit_id, 0xffff0001);
        assert_eq!(selections.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        assert_eq!(proposed.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        let original = &w.data[..32];
        for n in 0..32 {
            assert!(read(&original[..n], 2).is_err());
        }
        assert_eq!(read(original, 1), Err(Error::ResourceLimit));
        let mut bad = original.to_vec();
        bad[4..8].copy_from_slice(&15u32.to_le_bytes());
        assert_eq!(
            read(&bad, 2),
            Err(Error::Version {
                expected: 22,
                actual: 15
            })
        );
    }
}
