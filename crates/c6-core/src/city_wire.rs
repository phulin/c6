//! Bounded Linux XP2 City v44/v51 pre-component header and Districts v4.
//! Yield arrays require IndexSerializer version >0; the composed Core reader
//! admits version1. Other versions and arbitrary live global modes are unsupported.
//! This is wire transport, not a complete City codec, runtime definition remap,
//! gameplay state reconstruction, action execution, or turns. The next unframed
//! component after HeaderDistricts is Citizens, which is not consumed here.
//! Source: target a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa;
//! City writer 0x2890f30, reader 0x28913c0; Districts 0x2884a40/0x2884a90.
//! Nested writers: 0x2890910,0x2890bf0,0x2890d00,0x2890db0,0x2890e50,0x2890eb0.
//! Names follow matching target DWARF. Nested Yield/Happiness enums retain raw
//! indexes; Terrain/Improvement/GreatPersonClass enum overloads write hashes. Ordered Yield
//! pairs preserve duplicates/unknown hashes; Q8 fields retain signed raw bits.
use crate::unit::variants::{DecodeBudget, Limits as VariantLimits, VariantError, VariantMap};
use crate::unit::{HashValue, Limits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Version {
        expected: i32,
        actual: i32,
    },
    InvalidBool,
    /// Legacy reads carry omitted state that cannot be written as current state.
    UnsupportedLegacyWrite,
    Trailing,
    ResourceLimit,
    Variant(VariantError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux City wire input: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<VariantError> for Error {
    fn from(e: VariantError) -> Self {
        Self::Variant(e)
    }
}
fn reserve<T>(v: &mut Vec<T>, n: usize, limits: Limits) -> Result<(), Error> {
    if n.checked_mul(std::mem::size_of::<T>())
        .ok_or(Error::ResourceLimit)?
        > limits.max_bytes
    {
        return Err(Error::ResourceLimit);
    }
    v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)
}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let b = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(b)
    }
    fn claim(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            Err(Error::ResourceLimit)
        } else {
            Ok(())
        }
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(self.u32()? as i32)
    }
    fn bool(&mut self) -> Result<bool, Error> {
        match self.bytes(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::InvalidBool),
        }
    }
    fn version(&mut self, expected: i32) -> Result<(), Error> {
        let actual = self.i32()?;
        if actual != expected {
            Err(Error::Version { expected, actual })
        } else {
            Ok(())
        }
    }
    fn city_version(&mut self) -> Result<i32, Error> {
        let actual = self.i32()?;
        match actual {
            44 | 51 => Ok(actual),
            _ => Err(Error::Version {
                expected: 51,
                actual,
            }),
        }
    }
    fn array(&mut self) -> Result<[i32; 64], Error> {
        let mut a = [0; 64];
        for n in &mut a {
            *n = self.i32()?;
        }
        Ok(a)
    }
    fn list<T>(
        &mut self,
        min: usize,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let n = self.u32()? as usize;
        self.claim(n)?;
        if n > self.data.len().saturating_sub(self.pos) / min {
            return Err(Error::Truncated);
        }
        let mut v = Vec::new();
        reserve(&mut v, n, self.limits)?;
        for _ in 0..n {
            v.push(read(self)?);
        }
        Ok(v)
    }
    fn string(&mut self) -> Result<Vec<u8>, Error> {
        let n = self.u32()? as usize;
        self.claim(n)?;
        let mut v = Vec::new();
        reserve(&mut v, n, self.limits)?;
        v.extend_from_slice(self.bytes(n)?);
        Ok(v)
    }
    fn indexed(&mut self) -> Result<Vec<HashValue<i32>>, Error> {
        self.list(8, |r| {
            Ok(HashValue {
                definition_hash: r.u32()?,
                value: r.i32()?,
            })
        })
    }
    fn variants(&mut self) -> Result<VariantMap, Error> {
        let mut budget = DecodeBudget::new(VariantLimits {
            max_bytes: self.limits.max_bytes.saturating_sub(self.pos),
            max_items: self.limits.max_items.saturating_sub(self.items),
            max_depth: 64,
        });
        let (map, n) = VariantMap::decode_prefix(&self.data[self.pos..], &mut budget)?;
        self.claim(budget.items_used())?;
        self.bytes(n)?;
        Ok(map)
    }
}
struct Writer {
    data: Vec<u8>,
    items: usize,
    limits: Limits,
}
impl Writer {
    fn bytes(&mut self, b: &[u8]) -> Result<(), Error> {
        if b.len() > self.limits.max_bytes.saturating_sub(self.data.len()) {
            return Err(Error::ResourceLimit);
        }
        self.data
            .try_reserve(b.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.data.extend_from_slice(b);
        Ok(())
    }
    fn claim(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            Err(Error::ResourceLimit)
        } else {
            Ok(())
        }
    }
    fn u32(&mut self, n: u32) -> Result<(), Error> {
        self.bytes(&n.to_le_bytes())
    }
    fn i32(&mut self, n: i32) -> Result<(), Error> {
        self.u32(n as u32)
    }
    fn bool(&mut self, n: bool) -> Result<(), Error> {
        self.bytes(&[u8::from(n)])
    }
    fn array(&mut self, a: &[i32; 64]) -> Result<(), Error> {
        for &n in a {
            self.i32(n)?;
        }
        Ok(())
    }
    fn count(&mut self, n: usize) -> Result<(), Error> {
        self.claim(n)?;
        self.u32(u32::try_from(n).map_err(|_| Error::ResourceLimit)?)
    }
    fn string(&mut self, b: &[u8]) -> Result<(), Error> {
        self.count(b.len())?;
        self.bytes(b)
    }
    fn indexed(&mut self, v: &[HashValue<i32>]) -> Result<(), Error> {
        self.count(v.len())?;
        for p in v {
            self.u32(p.definition_hash)?;
            self.i32(p.value)?;
        }
        Ok(())
    }
    fn variants(&mut self, v: &VariantMap) -> Result<(), Error> {
        let (bytes, n) = v.encode(VariantLimits {
            max_bytes: self.limits.max_bytes.saturating_sub(self.data.len()),
            max_items: self.limits.max_items.saturating_sub(self.items),
            max_depth: 64,
        })?;
        self.claim(n)?;
        self.bytes(&bytes)
    }
}
trait Wire: Sized {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HappinessModifiers {
    /// Writer 0x2890d0a saves the raw ordinal through scalar int, whereas
    /// reader 0x2890d6a invokes HappinessTypes HashToIndex. This wire value
    /// preserves that asymmetry; it is not an applied runtime Happiness index.
    pub happiness_type_index: i32,
    pub great_person_class_hash: u32,
    pub yield_type_index: i32,
    pub yield_amount: i32,
    pub great_person_amount: i32,
}
impl Wire for HappinessModifiers {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            happiness_type_index: r.i32()?,
            great_person_class_hash: r.u32()?,
            yield_type_index: r.i32()?,
            yield_amount: r.i32()?,
            great_person_amount: r.i32()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(self.happiness_type_index)?;
        w.u32(self.great_person_class_hash)?;
        w.i32(self.yield_type_index)?;
        w.i32(self.yield_amount)?;
        w.i32(self.great_person_amount)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerrainModifiers {
    pub terrain_hash: u32,
    pub adjacent_improvement_hash: u32,
    pub yield_type_index: i32,
    pub yield_amount: i32,
}
impl Wire for TerrainModifiers {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            terrain_hash: r.u32()?,
            adjacent_improvement_hash: r.u32()?,
            yield_type_index: r.i32()?,
            yield_amount: r.i32()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.u32(self.terrain_hash)?;
        w.u32(self.adjacent_improvement_hash)?;
        w.i32(self.yield_type_index)?;
        w.i32(self.yield_amount)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImprovementValidTerrain {
    pub terrain_hash: u32,
    pub improvement_hash: u32,
}
impl Wire for ImprovementValidTerrain {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            terrain_hash: r.u32()?,
            improvement_hash: r.u32()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.u32(self.terrain_hash)?;
        w.u32(self.improvement_hash)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerrainClassYieldImprovementModifiers {
    pub improvement_hash: u32,
    pub yield_type_index: i32,
    pub amount: i32,
}
impl Wire for TerrainClassYieldImprovementModifiers {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            improvement_hash: r.u32()?,
            yield_type_index: r.i32()?,
            amount: r.i32()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.u32(self.improvement_hash)?;
        w.i32(self.yield_type_index)?;
        w.i32(self.amount)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicAdjacencyBonus {
    pub district_hash: u32,
    pub feature_hash: u32,
    pub terrain_hash: u32,
    /// None means the nested44 read supplies native index -1 without a hash.
    pub improvement_hash: Option<u32>,
    pub river: bool,
    pub adjacent_district: bool,
    pub appeal: i32,
    pub yield_hash: u32,
    pub tiles_required: i32,
    pub amount: i32,
    pub description: Vec<u8>,
}
impl Wire for DynamicAdjacencyBonus {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let version = r.city_version()?;
        Ok(Self {
            district_hash: r.u32()?,
            feature_hash: r.u32()?,
            terrain_hash: r.u32()?,
            improvement_hash: if version >= 51 { Some(r.u32()?) } else { None },
            river: r.bool()?,
            adjacent_district: r.bool()?,
            appeal: r.i32()?,
            yield_hash: r.u32()?,
            tiles_required: r.i32()?,
            amount: r.i32()?,
            description: r.string()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let improvement = self.improvement_hash.ok_or(Error::UnsupportedLegacyWrite)?;
        w.i32(51)?;
        w.u32(self.district_hash)?;
        w.u32(self.feature_hash)?;
        w.u32(self.terrain_hash)?;
        w.u32(improvement)?;
        w.bool(self.river)?;
        w.bool(self.adjacent_district)?;
        w.i32(self.appeal)?;
        w.u32(self.yield_hash)?;
        w.i32(self.tiles_required)?;
        w.i32(self.amount)?;
        w.string(&self.description)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YieldPerTerrainType {
    pub terrain_hash: u32,
    pub yield_hash: u32,
    pub amount_q8: i32,
}
impl Wire for YieldPerTerrainType {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(51)?;
        Ok(Self {
            terrain_hash: r.u32()?,
            yield_hash: r.u32()?,
            amount_q8: r.i32()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(51)?;
        w.u32(self.terrain_hash)?;
        w.u32(self.yield_hash)?;
        w.i32(self.amount_q8)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Districts {
    pub district_ids: Vec<u32>,
    pub extra_districts: i32,
}
impl Wire for Districts {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(4)?;
        Ok(Self {
            district_ids: r.list(4, |r| r.u32())?,
            extra_districts: r.i32()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(4)?;
        w.count(self.district_ids.len())?;
        for &n in &self.district_ids {
            w.u32(n)?;
        }
        w.i32(self.extra_districts)?;
        Ok(())
    }
}

/// Read effect of a version-omitted field; an empty wire list is not a
/// reconstructed native array or a known empty retained baseline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OmittedFieldEffect {
    Present,
    ZeroResetDefinitionCountUnknown,
    UnknownRetainedBaseline,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Header {
    pub wire_version: i32,
    /// Version37..44 temporary Q8 pairs; native discards this list after reading.
    pub deprecated_yield_q8: Vec<HashValue<i32>>,
    pub id: u32,
    pub location_x: i32,
    pub location_y: i32,
    pub owner: i32,
    pub original_owner: i32,
    pub owner_before_occupation: i32,
    pub liberator: i32,
    pub city_ai_id: u32,
    pub population: i32,
    pub capital: bool,
    pub original_capital: bool,
    pub last_transfer_type: i32,
    pub just_conquered_from: i32,
    pub just_rebelled_from: i32,
    pub last_barbarian_invasion_turn: i32,
    pub occupied: bool,
    pub last_warmonger_penalty: [i32; 64],
    pub production_unit_changes: i32,
    pub production_building_changes: i32,
    pub production_district_changes: i32,
    pub wonder_envoy: i32,
    pub delayed_death: bool,
    pub transferring: bool,
    pub yield_modifiers: Vec<HashValue<i32>>,
    pub follower_yield_modifiers: Vec<HashValue<i32>>,
    pub yield_modifiers_per_governor_title_q8: Vec<HashValue<i32>>,
    pub yield_changes: Vec<HashValue<i32>>,
    pub yield_per_district: Vec<HashValue<i32>>,
    pub extra_yield_per_population_q8: Vec<HashValue<i32>>,
    pub allow_incoming_regional_stacking_counts: Vec<HashValue<i32>>,
    pub yield_per_unit_complete: Vec<HashValue<i32>>,
    pub yield_per_building_complete: Vec<HashValue<i32>>,
    pub yield_per_wonder_complete: Vec<HashValue<i32>>,
    pub yield_per_terrain_class_cities: Vec<HashValue<i32>>,
    pub project_yield_q8: Vec<HashValue<i32>>,
    pub yield_modifier_from_faith_city: Vec<HashValue<i32>>,
    pub happiness_modifiers: Vec<HappinessModifiers>,
    pub terrain_modifiers: Vec<TerrainModifiers>,
    pub improvement_valid_terrain: Vec<ImprovementValidTerrain>,
    pub terrain_class_yield_improvement_modifiers: Vec<TerrainClassYieldImprovementModifiers>,
    pub project_yield_project_hash: u32,
    pub housing_per_district: i32,
    pub dynamic_bonuses: Vec<DynamicAdjacencyBonus>,
    pub yield_per_terrain_type: Vec<YieldPerTerrainType>,
    pub max_unit_level: i32,
    pub unit_experience_modifier_q8: i32,
    pub attacks_per_turn: i32,
    pub great_person_point_bonus_percent: i32,
    pub settler_consume_pop: bool,
    pub yield_per_flood: Vec<HashValue<i32>>,
    pub espionage_defense_bonus: i32,
    pub project_production: bool,
    pub prevent_bypass_outer_defenses: i32,
    pub prevent_melee_attack_outer_defenses: i32,
    pub name: Vec<u8>,
    pub attributes: VariantMap,
}
impl Header {
    /// Applies to per-building, per-wonder and faith-city yield lists.
    pub fn omitted_yield_effect(&self) -> OmittedFieldEffect {
        if self.wire_version == 44 {
            OmittedFieldEffect::ZeroResetDefinitionCountUnknown
        } else {
            OmittedFieldEffect::Present
        }
    }
    pub fn terrain_yield_effect(&self) -> OmittedFieldEffect {
        if self.wire_version == 44 {
            OmittedFieldEffect::UnknownRetainedBaseline
        } else {
            OmittedFieldEffect::Present
        }
    }
    pub fn powered_yield_effect(&self) -> OmittedFieldEffect {
        self.omitted_yield_effect()
    }
}
impl Wire for Header {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let version = r.city_version()?;
        Ok(Self {
            wire_version: version,
            id: r.u32()?,
            location_x: r.i32()?,
            location_y: r.i32()?,
            owner: r.i32()?,
            original_owner: r.i32()?,
            owner_before_occupation: r.i32()?,
            liberator: r.i32()?,
            city_ai_id: r.u32()?,
            population: r.i32()?,
            capital: r.bool()?,
            original_capital: r.bool()?,
            last_transfer_type: r.i32()?,
            just_conquered_from: r.i32()?,
            just_rebelled_from: r.i32()?,
            last_barbarian_invasion_turn: r.i32()?,
            occupied: r.bool()?,
            last_warmonger_penalty: r.array()?,
            production_unit_changes: r.i32()?,
            production_building_changes: r.i32()?,
            production_district_changes: r.i32()?,
            wonder_envoy: r.i32()?,
            delayed_death: r.bool()?,
            transferring: r.bool()?,
            deprecated_yield_q8: if version == 44 {
                r.indexed()?
            } else {
                Vec::new()
            },
            yield_modifiers: r.indexed()?,
            follower_yield_modifiers: r.indexed()?,
            yield_modifiers_per_governor_title_q8: r.indexed()?,
            yield_changes: r.indexed()?,
            yield_per_district: r.indexed()?,
            extra_yield_per_population_q8: r.indexed()?,
            allow_incoming_regional_stacking_counts: r.indexed()?,
            yield_per_unit_complete: r.indexed()?,
            yield_per_building_complete: if version == 51 {
                r.indexed()?
            } else {
                Vec::new()
            },
            yield_per_wonder_complete: if version == 51 {
                r.indexed()?
            } else {
                Vec::new()
            },
            yield_per_terrain_class_cities: r.indexed()?,
            project_yield_q8: r.indexed()?,
            yield_modifier_from_faith_city: if version == 51 {
                r.indexed()?
            } else {
                Vec::new()
            },
            happiness_modifiers: r.list(20, HappinessModifiers::read)?,
            terrain_modifiers: r.list(16, TerrainModifiers::read)?,
            improvement_valid_terrain: r.list(8, ImprovementValidTerrain::read)?,
            terrain_class_yield_improvement_modifiers: r
                .list(12, TerrainClassYieldImprovementModifiers::read)?,
            project_yield_project_hash: r.u32()?,
            housing_per_district: r.i32()?,
            dynamic_bonuses: r.list(38, DynamicAdjacencyBonus::read)?,
            yield_per_terrain_type: if version == 51 {
                r.list(16, YieldPerTerrainType::read)?
            } else {
                Vec::new()
            },
            max_unit_level: r.i32()?,
            unit_experience_modifier_q8: r.i32()?,
            attacks_per_turn: r.i32()?,
            great_person_point_bonus_percent: r.i32()?,
            settler_consume_pop: r.bool()?,
            yield_per_flood: r.indexed()?,
            espionage_defense_bonus: r.i32()?,
            project_production: r.bool()?,
            prevent_bypass_outer_defenses: if version == 51 { r.i32()? } else { 0 },
            prevent_melee_attack_outer_defenses: if version == 51 { r.i32()? } else { 0 },
            name: r.string()?,
            attributes: r.variants()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 51 || !self.deprecated_yield_q8.is_empty() {
            return Err(Error::UnsupportedLegacyWrite);
        }
        w.i32(51)?;
        w.u32(self.id)?;
        w.i32(self.location_x)?;
        w.i32(self.location_y)?;
        w.i32(self.owner)?;
        w.i32(self.original_owner)?;
        w.i32(self.owner_before_occupation)?;
        w.i32(self.liberator)?;
        w.u32(self.city_ai_id)?;
        w.i32(self.population)?;
        w.bool(self.capital)?;
        w.bool(self.original_capital)?;
        w.i32(self.last_transfer_type)?;
        w.i32(self.just_conquered_from)?;
        w.i32(self.just_rebelled_from)?;
        w.i32(self.last_barbarian_invasion_turn)?;
        w.bool(self.occupied)?;
        w.array(&self.last_warmonger_penalty)?;
        w.i32(self.production_unit_changes)?;
        w.i32(self.production_building_changes)?;
        w.i32(self.production_district_changes)?;
        w.i32(self.wonder_envoy)?;
        w.bool(self.delayed_death)?;
        w.bool(self.transferring)?;
        w.indexed(&self.yield_modifiers)?;
        w.indexed(&self.follower_yield_modifiers)?;
        w.indexed(&self.yield_modifiers_per_governor_title_q8)?;
        w.indexed(&self.yield_changes)?;
        w.indexed(&self.yield_per_district)?;
        w.indexed(&self.extra_yield_per_population_q8)?;
        w.indexed(&self.allow_incoming_regional_stacking_counts)?;
        w.indexed(&self.yield_per_unit_complete)?;
        w.indexed(&self.yield_per_building_complete)?;
        w.indexed(&self.yield_per_wonder_complete)?;
        w.indexed(&self.yield_per_terrain_class_cities)?;
        w.indexed(&self.project_yield_q8)?;
        w.indexed(&self.yield_modifier_from_faith_city)?;
        w.count(self.happiness_modifiers.len())?;
        for n in &self.happiness_modifiers {
            n.write(w)?;
        }
        w.count(self.terrain_modifiers.len())?;
        for n in &self.terrain_modifiers {
            n.write(w)?;
        }
        w.count(self.improvement_valid_terrain.len())?;
        for n in &self.improvement_valid_terrain {
            n.write(w)?;
        }
        w.count(self.terrain_class_yield_improvement_modifiers.len())?;
        for n in &self.terrain_class_yield_improvement_modifiers {
            n.write(w)?;
        }
        w.u32(self.project_yield_project_hash)?;
        w.i32(self.housing_per_district)?;
        w.count(self.dynamic_bonuses.len())?;
        for n in &self.dynamic_bonuses {
            n.write(w)?;
        }
        w.count(self.yield_per_terrain_type.len())?;
        for n in &self.yield_per_terrain_type {
            n.write(w)?;
        }
        w.i32(self.max_unit_level)?;
        w.i32(self.unit_experience_modifier_q8)?;
        w.i32(self.attacks_per_turn)?;
        w.i32(self.great_person_point_bonus_percent)?;
        w.bool(self.settler_consume_pop)?;
        w.indexed(&self.yield_per_flood)?;
        w.i32(self.espionage_defense_bonus)?;
        w.bool(self.project_production)?;
        w.i32(self.prevent_bypass_outer_defenses)?;
        w.i32(self.prevent_melee_attack_outer_defenses)?;
        w.string(&self.name)?;
        w.variants(&self.attributes)?;
        Ok(())
    }
}

/// Precisely the City prefix ending at the next unframed Citizens component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeaderDistricts {
    pub header: Header,
    pub districts: Districts,
}
impl Wire for HeaderDistricts {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            header: Header::read(r)?,
            districts: Districts::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.header.write(w)?;
        self.districts.write(w)
    }
}
macro_rules! codec {
    ($t:ty) => {
        impl $t {
            pub fn decode_prefix(data: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
                let (value, consumed, _) = Self::decode_prefix_with_items(data, limits)?;
                Ok((value, consumed))
            }
            /// Decode one prefix and return its aggregate dynamic item charge.
            pub fn decode_prefix_with_items(
                data: &[u8],
                limits: Limits,
            ) -> Result<(Self, usize, usize), Error> {
                let mut r = Reader {
                    data,
                    pos: 0,
                    items: 0,
                    limits,
                };
                let value = Self::read(&mut r)?;
                Ok((value, r.pos, r.items))
            }
            pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
                let (v, n) = Self::decode_prefix(data, limits)?;
                if n != data.len() {
                    Err(Error::Trailing)
                } else {
                    Ok(v)
                }
            }
            pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
                Ok(self.encode_with_items(limits)?.0)
            }
            /// Encode this boundary and return its aggregate dynamic item charge.
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
codec!(Header);
codec!(Districts);
codec!(HeaderDistricts);

#[cfg(test)]
mod tests {
    use super::*;

    // Independent literal current-header grammar: no component bytes included.
    fn empty_header_wire() -> Vec<u8> {
        header_wire(51)
    }
    fn header_wire(version: i32) -> Vec<u8> {
        let mut bytes = Vec::new();
        let put = |bytes: &mut Vec<u8>, value: i32| bytes.extend(value.to_le_bytes());
        put(&mut bytes, version);
        for value in [0x12345678, -3, 7, 64, 63, -1, -1, 17, 6] {
            put(&mut bytes, value);
        }
        bytes.extend([0, 1]); // capital, original capital
        for value in [-1, 3, 4, -10] {
            put(&mut bytes, value);
        }
        bytes.push(1); // occupied
        for value in 0..64 {
            put(&mut bytes, value - 32);
        }
        for value in [5, 6, 7, 8] {
            put(&mut bytes, value);
        }
        bytes.extend([1, 0]); // delayed death, transferring
        if version == 44 {
            put(&mut bytes, 2); // discarded Q8 list: duplicate unknown hashes, raw bits
            for value in [-1, i32::MIN, -1, -257] {
                put(&mut bytes, value);
            }
        }
        for _ in 0..if version == 44 { 10 } else { 13 } {
            put(&mut bytes, 0);
        } // yield index lists
        for _ in 0..4 {
            put(&mut bytes, 0);
        } // small nested record vectors
        put(&mut bytes, -1); // project hash
        put(&mut bytes, 9); // housing per district
        put(&mut bytes, 0); // dynamic
        if version == 51 {
            put(&mut bytes, 0);
        } // terrain yield vector
        for value in [10, -257, 11, 12] {
            put(&mut bytes, value);
        }
        bytes.push(1); // settler consumes population
        put(&mut bytes, 0); // flood index list
        put(&mut bytes, 13); // espionage defense
        bytes.push(0); // project production
        if version == 51 {
            put(&mut bytes, 14);
            put(&mut bytes, 15);
        }
        put(&mut bytes, 3); // byte string; no terminator or UTF-8 assumption
        bytes.extend([b'X', 0, 0xff]);
        put(&mut bytes, 1); // VariantMap scalar tree
        put(&mut bytes, -1); // unsigned key
        put(&mut bytes, i32::MIN); // exact union bits
        put(&mut bytes, 0); // empty array tree
        bytes
    }
    #[test]
    fn legacy44_omissions_discarded_pairs_and_exact_boundary() {
        let mut bytes = header_wire(44);
        let end = bytes.len();
        bytes.extend([0xaa, 0xbb]);
        let (value, n, items) =
            Header::decode_prefix_with_items(&bytes, Limits::default()).unwrap();
        assert_eq!((n, items), (end, 6));
        assert_eq!(value.wire_version, 44);
        assert_eq!(
            value.deprecated_yield_q8,
            [
                HashValue {
                    definition_hash: u32::MAX,
                    value: i32::MIN
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: -257
                },
            ]
        );
        assert!(value.yield_modifiers.is_empty());
        assert!(value.yield_per_building_complete.is_empty());
        assert!(value.yield_per_wonder_complete.is_empty());
        assert!(value.yield_modifier_from_faith_city.is_empty());
        assert_eq!(
            value.omitted_yield_effect(),
            OmittedFieldEffect::ZeroResetDefinitionCountUnknown
        );
        assert_eq!(
            value.powered_yield_effect(),
            OmittedFieldEffect::ZeroResetDefinitionCountUnknown
        );
        assert_eq!(
            value.terrain_yield_effect(),
            OmittedFieldEffect::UnknownRetainedBaseline
        );
        assert_eq!(
            (
                value.prevent_bypass_outer_defenses,
                value.prevent_melee_attack_outer_defenses
            ),
            (0, 0)
        );
        assert_eq!(value.name, [b'X', 0, 0xff]);
        assert_eq!(
            value.encode(Limits::default()),
            Err(Error::UnsupportedLegacyWrite)
        );
        for n in 0..end {
            assert!(
                Header::decode_prefix(&bytes[..n], Limits::default()).is_err(),
                "{n}"
            );
        }
        let exact = Limits {
            max_bytes: end,
            max_items: items,
        };
        assert!(Header::decode_prefix(&bytes, exact).is_ok());
        assert_eq!(
            Header::decode_prefix(
                &bytes,
                Limits {
                    max_items: items - 1,
                    ..exact
                }
            ),
            Err(Error::Variant(VariantError::Limit))
        );
    }
    #[test]
    fn nested44_has_independent_version_and_no_improvement_hash() {
        let mut bytes = Vec::new();
        for value in [44i32, 11, 12, 13] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([1, 0]);
        for value in [-15i32, 16, 17, -18, 0] {
            bytes.extend(value.to_le_bytes());
        }
        let mut r = Reader {
            data: &bytes,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        let value = DynamicAdjacencyBonus::read(&mut r).unwrap();
        assert_eq!(r.pos, bytes.len());
        assert_eq!(value.improvement_hash, None);
        assert_eq!(
            (value.appeal, value.yield_hash, value.amount),
            (-15, 16, -18)
        );
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits: Limits::default(),
        };
        assert_eq!(value.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        // Outer51 does not override the nested44 version.
        let mut outer = empty_header_wire();
        // Independent literal fixture: dynamic vector lies after 13 empty yield
        // lists, four empty nested lists, project hash and housing.
        let dynamic = 4 + 9 * 4 + 2 + 4 * 4 + 1 + 64 * 4 + 4 * 4 + 2 + 13 * 4 + 4 * 4 + 2 * 4;
        outer[dynamic..dynamic + 4].copy_from_slice(&1u32.to_le_bytes());
        outer.splice(dynamic + 4..dynamic + 4, bytes);
        let h = Header::decode(&outer, Limits::default()).unwrap();
        assert_eq!(h.wire_version, 51);
        assert_eq!(h.dynamic_bonuses, [value]);
    }
    #[test]
    fn literal_header_and_district_boundary_preserve_scalars_and_bytes() {
        let mut bytes = empty_header_wire();
        let header_end = bytes.len();
        for n in [4u32, 3, 0x10001, u32::MAX, 0x10001, (-7i32) as u32] {
            bytes.extend(n.to_le_bytes());
        }
        let end = bytes.len();
        bytes.extend([0xaa, 0xbb]); // next unframed component is untouched
        let limits = Limits::default();
        let (value, consumed) = HeaderDistricts::decode_prefix(&bytes, limits).unwrap();
        assert_eq!(consumed, end);
        assert_eq!(value.header.id, 0x12345678);
        assert_eq!((value.header.location_x, value.header.location_y), (-3, 7));
        assert_eq!(value.header.owner, 64);
        assert_eq!(value.header.city_ai_id, 17);
        assert_eq!(value.header.last_warmonger_penalty[63], 31);
        assert_eq!(value.header.unit_experience_modifier_q8, -257);
        assert_eq!(value.header.name, [b'X', 0, 0xff]);
        assert_eq!(value.header.attributes.scalars, [(u32::MAX, 0x80000000)]);
        assert_eq!(value.districts.district_ids, [0x10001, u32::MAX, 0x10001]);
        assert_eq!(value.districts.extra_districts, -7);
        assert_eq!(value.encode(limits).unwrap(), bytes[..end]);
        assert_eq!(Header::decode_prefix(&bytes, limits).unwrap().1, header_end);
        assert_eq!(
            HeaderDistricts::decode(&bytes, limits),
            Err(Error::Trailing)
        );
        for n in 0..end {
            assert!(
                HeaderDistricts::decode_prefix(&bytes[..n], limits).is_err(),
                "{n}"
            );
        }
    }
    #[test]
    fn nested_records_and_ordered_yield_pairs_retain_raw_representations() {
        let limits = Limits::default();
        let mut header = Header::decode(&empty_header_wire(), limits).unwrap();
        header.yield_modifiers = vec![
            HashValue {
                definition_hash: 0xffffffff,
                value: -8,
            },
            HashValue {
                definition_hash: 0xffffffff,
                value: 9,
            },
        ];
        header.project_yield_q8 = vec![HashValue {
            definition_hash: 5,
            value: i32::MIN,
        }];
        header.happiness_modifiers.push(HappinessModifiers {
            happiness_type_index: -1,
            great_person_class_hash: 27,
            yield_type_index: 99,
            yield_amount: -123,
            great_person_amount: i32::MAX,
        });
        header.terrain_modifiers.push(TerrainModifiers {
            terrain_hash: u32::MAX,
            adjacent_improvement_hash: 4,
            yield_type_index: 5,
            yield_amount: -6,
        });
        header
            .improvement_valid_terrain
            .push(ImprovementValidTerrain {
                terrain_hash: 7,
                improvement_hash: 8,
            });
        header.terrain_class_yield_improvement_modifiers.push(
            TerrainClassYieldImprovementModifiers {
                improvement_hash: 9,
                yield_type_index: -1,
                amount: 10,
            },
        );
        header.dynamic_bonuses.push(DynamicAdjacencyBonus {
            district_hash: 11,
            feature_hash: 12,
            terrain_hash: 13,
            improvement_hash: Some(14),
            river: true,
            adjacent_district: false,
            appeal: -15,
            yield_hash: 16,
            tiles_required: 17,
            amount: -18,
            description: vec![0xff, b'Y'],
        });
        header.yield_per_terrain_type.push(YieldPerTerrainType {
            terrain_hash: 19,
            yield_hash: 20,
            amount_q8: -513,
        });
        let wire = header.encode(limits).unwrap();
        assert_eq!(Header::decode(&wire, limits).unwrap(), header);
        // Independent nested writer literal verifies Feature before Terrain,
        // current nested version, hash fields, one-byte bools and string count.
        let mut expected = Vec::new();
        for n in [51i32, 11, 12, 13, 14] {
            expected.extend(n.to_le_bytes());
        }
        expected.extend([1, 0]);
        for n in [-15i32, 16, 17, -18, 2] {
            expected.extend(n.to_le_bytes());
        }
        expected.extend([0xff, b'Y']);
        let mut writer = Writer {
            data: Vec::new(),
            items: 0,
            limits,
        };
        header.dynamic_bonuses[0].write(&mut writer).unwrap();
        assert_eq!(writer.data, expected);
        expected[0] = 50;
        let mut reader = Reader {
            data: &expected,
            pos: 0,
            items: 0,
            limits,
        };
        assert_eq!(
            DynamicAdjacencyBonus::read(&mut reader),
            Err(Error::Version {
                expected: 51,
                actual: 50
            })
        );
    }
    #[test]
    fn item_charge_api_composes_prefixes_without_charging_the_tail() {
        let limits = Limits::default();
        let mut bytes = empty_header_wire();
        let header_end = bytes.len();
        for n in [4u32, 2, 0x10001, 0x20002, 3] {
            bytes.extend(n.to_le_bytes());
        }
        let end = bytes.len();
        bytes.extend([0xaa, 0xbb]);
        let (header, consumed, header_items) =
            Header::decode_prefix_with_items(&bytes, limits).unwrap();
        assert_eq!((consumed, header_items), (header_end, 4));
        assert_eq!(
            header.encode_with_items(limits).unwrap(),
            (bytes[..header_end].to_vec(), 4)
        );
        let remaining = Limits {
            max_bytes: limits.max_bytes - consumed,
            max_items: 2,
        };
        let (districts, district_bytes, district_items) =
            Districts::decode_prefix_with_items(&bytes[consumed..], remaining).unwrap();
        assert_eq!((district_bytes, district_items), (end - header_end, 2));
        assert_eq!(
            districts.encode_with_items(remaining).unwrap(),
            (bytes[header_end..end].to_vec(), 2)
        );
        let exact = Limits {
            max_bytes: end,
            max_items: header_items + district_items,
        };
        let (combined, n, items) =
            HeaderDistricts::decode_prefix_with_items(&bytes, exact).unwrap();
        assert_eq!((n, items), (end, 6));
        assert_eq!(combined.header, header);
        assert_eq!(combined.districts, districts);
        assert_eq!(
            combined.encode_with_items(exact).unwrap(),
            (bytes[..end].to_vec(), 6)
        );
        let insufficient = Limits {
            max_items: 5,
            ..exact
        };
        assert!(HeaderDistricts::decode_prefix_with_items(&bytes, insufficient).is_err());
        assert!(combined.encode_with_items(insufficient).is_err());
    }
    #[test]
    fn fail_closed_versions_bools_counts_and_aggregate_limits() {
        let wire = empty_header_wire();
        let limits = Limits::default();
        let mut bad = wire.clone();
        bad[0] = 50;
        assert_eq!(
            Header::decode(&bad, limits),
            Err(Error::Version {
                expected: 51,
                actual: 50
            })
        );
        bad = wire.clone();
        bad[40] = 2;
        assert_eq!(Header::decode(&bad, limits), Err(Error::InvalidBool));
        let value = Header::decode(&wire, limits).unwrap();
        // Name bytes (3) and attribute entry (1) share the total item budget.
        let exact = Limits {
            max_items: 4,
            max_bytes: wire.len(),
        };
        assert_eq!(value.encode(exact).unwrap(), wire);
        assert_eq!(Header::decode(&wire, exact).unwrap(), value);
        for max_items in 0..4 {
            let bounded = Limits { max_items, ..exact };
            assert!(Header::decode(&wire, bounded).is_err());
            assert!(value.encode(bounded).is_err());
        }
        let bounded = Limits {
            max_bytes: wire.len() - 1,
            ..limits
        };
        assert!(Header::decode(&wire, bounded).is_err());
        assert!(value.encode(bounded).is_err());
        let district = [4u32, u32::MAX];
        let bytes: Vec<_> = district.into_iter().flat_map(u32::to_le_bytes).collect();
        assert_eq!(Districts::decode(&bytes, limits), Err(Error::ResourceLimit));
        let legacy = 3i32.to_le_bytes();
        assert_eq!(
            Districts::decode(&legacy, limits),
            Err(Error::Version {
                expected: 4,
                actual: 3
            })
        );
    }
}
