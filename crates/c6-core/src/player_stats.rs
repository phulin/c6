//! Linux XP2 Player.Stats v23/v27 wire transport and source-defined legacy defaults.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::tracked_value::TrackedValue;
use crate::unit::{HashValue, Limits};

pub const VERSION: u32 = 27;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    UnsupportedLegacyWrite,
    Truncated,
    Trailing,
    ResourceLimit,
    InvalidBool(u8),
    InvalidContainer,
    TrackedValue(crate::tracked_value::Error),
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Stats wire: {self:?}")
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
    fn version(&mut self) -> Result<u32, Error> {
        let actual = u32::read(self)?;
        if actual != 23 && actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        Ok(actual)
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
impl Wire for bool {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        match r.raw(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            x => Err(Error::InvalidBool(x)),
        }
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.raw(&[u8::from(*self)])
    }
}
impl Wire for HashValue<i32> {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            definition_hash: u32::read(r)?,
            value: i32::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.definition_hash.write(w)?;
        self.value.write(w)
    }
}
/// Native WriteArray framing requires exactly 64 entries for these seven fields.
impl Wire for [i32; 64] {
    const MIN: usize = 260;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        if u32::read(r)? != 64 {
            return Err(Error::InvalidContainer);
        }
        r.charge(64)?;
        let mut values = [0; 64];
        for value in &mut values {
            *value = i32::read(r)?;
        }
        Ok(values)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.charge(64)?;
        64u32.write(w)?;
        for value in self {
            value.write(w)?;
        }
        Ok(())
    }
}
impl Wire for TrackedValue {
    const MIN: usize = 15;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let (v, n, items) = Self::decode_prefix_with_items(&r.data[r.pos..], r.remaining())
            .map_err(Error::TrackedValue)?;
        r.raw(n)?;
        r.charge(items)?;
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let limits = Limits {
            max_bytes: w.limits.max_bytes - w.data.len(),
            max_items: w.limits.max_items - w.items,
        };
        let (bytes, items) = self
            .encode_with_items(limits)
            .map_err(Error::TrackedValue)?;
        w.charge(items)?;
        w.raw(&bytes)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stats {
    pub wire_version: u32,
    /// Exact v23 integer before native doubling and Q8 conversion; absent at v27.
    pub legacy_diplomatic_victory_points: Option<i32>,
    pub max_melee_strength_trained: i32,
    pub max_ranged_strength_trained: i32,
    pub max_bombard_strength_trained: i32,
    pub military_strength: i32,
    pub military_strength_from_gold: i32,
    pub air_military_strength: i32,
    pub land_military_strength: i32,
    pub naval_military_strength: i32,
    pub barbarians_killed: i32,
    pub barbarian_camps_cleared: i32,
    pub tiles_pillaged: i32,
    pub wmd_city_hits: i32,
    pub cities_revolted: i32,
    pub cities_annexed: i32,
    pub total_cities_lost_to_disloyalty: i32,
    pub total_cities_gained_from_disloyalty: i32,
    pub total_number_of_alliances_formed: i32,
    pub total_number_of_emergency_alliances_formed: i32,
    pub gold_spent_levying: i32,
    pub emergencies_joined: i32,
    pub total_governors_appointed: i32,
    pub total_governor_assignments: i32,
    pub diplomatic_victory_points: TrackedValue,
    pub last_turn_favor: i32,
    pub current_turn_favor: i32,
    pub lifetime_favor: i32,
    pub science_victory_points_q8: i32,
    pub science_victory_points_per_turn_q8: i32,
    pub num_units_constructed: Vec<HashValue<i32>>,
    pub num_projects_advanced: Vec<HashValue<i32>>,
    pub wonders_lost_plots: Vec<i32>,
    pub cached_yields_q8: Vec<HashValue<i32>>,
    pub removed_feature_counts: [i32; 64],
    pub national_park_counts: [i32; 64],
    pub placed_forest_counts: [i32; 64],
    pub known_wmd_counts: [i32; 64],
    pub cleared_bandit_camp_counts: [i32; 64],
    pub city_state_war_counts: [i32; 64],
    pub players_revived: [i32; 64],
    pub fertility_added_by_droughts: i32,
    pub tiles_damaged_by_droughts: i32,
    pub population_lost_by_droughts: i32,
    pub units_killed_by_droughts: i32,
    pub fertility_added_by_floods: i32,
    pub tiles_damaged_by_floods: i32,
    pub population_lost_by_floods: i32,
    pub units_killed_by_floods: i32,
    pub fertility_added_by_storms: i32,
    pub tiles_damaged_by_storms: i32,
    pub population_lost_by_storms: i32,
    pub units_killed_by_storms: i32,
    pub fertility_added_by_volcanos: i32,
    pub tiles_damaged_by_volcanos: i32,
    pub population_lost_by_volcanos: i32,
    pub units_killed_by_volcanos: i32,
}
impl Wire for Stats {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = r.version()?;
        let max_melee_strength_trained = <i32 as Wire>::read(r)?;
        let max_ranged_strength_trained = <i32 as Wire>::read(r)?;
        let max_bombard_strength_trained = <i32 as Wire>::read(r)?;
        let military_strength = <i32 as Wire>::read(r)?;
        let military_strength_from_gold = <i32 as Wire>::read(r)?;
        let air_military_strength = <i32 as Wire>::read(r)?;
        let land_military_strength = <i32 as Wire>::read(r)?;
        let naval_military_strength = <i32 as Wire>::read(r)?;
        let barbarians_killed = <i32 as Wire>::read(r)?;
        let barbarian_camps_cleared = <i32 as Wire>::read(r)?;
        let tiles_pillaged = <i32 as Wire>::read(r)?;
        let wmd_city_hits = <i32 as Wire>::read(r)?;
        let cities_revolted = <i32 as Wire>::read(r)?;
        let cities_annexed = <i32 as Wire>::read(r)?;
        let total_cities_lost_to_disloyalty = <i32 as Wire>::read(r)?;
        let total_cities_gained_from_disloyalty = <i32 as Wire>::read(r)?;
        let total_number_of_alliances_formed = <i32 as Wire>::read(r)?;
        let total_number_of_emergency_alliances_formed = <i32 as Wire>::read(r)?;
        let gold_spent_levying = <i32 as Wire>::read(r)?;
        let emergencies_joined = <i32 as Wire>::read(r)?;
        let total_governors_appointed = <i32 as Wire>::read(r)?;
        let total_governor_assignments = <i32 as Wire>::read(r)?;
        let legacy_diplomatic_victory_points = if wire_version == 23 {
            Some(i32::read(r)?)
        } else {
            None
        };
        let diplomatic_victory_points = match legacy_diplomatic_victory_points {
            Some(value) => TrackedValue {
                value_q8: value.wrapping_shl(9),
                include_steps: true,
                min_q8: None,
                max_q8: None,
                steps: Vec::new(),
            },
            None => TrackedValue::read(r)?,
        };
        let last_turn_favor = <i32 as Wire>::read(r)?;
        let current_turn_favor = <i32 as Wire>::read(r)?;
        let lifetime_favor = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let science_victory_points_q8 = <i32 as Wire>::read(r)?;
        let science_victory_points_per_turn_q8 = <i32 as Wire>::read(r)?;
        let num_units_constructed = <Vec<HashValue<i32>> as Wire>::read(r)?;
        let num_projects_advanced = <Vec<HashValue<i32>> as Wire>::read(r)?;
        let wonders_lost_plots = <Vec<i32> as Wire>::read(r)?;
        let cached_yields_q8 = <Vec<HashValue<i32>> as Wire>::read(r)?;
        let removed_feature_counts = <[i32; 64] as Wire>::read(r)?;
        let national_park_counts = <[i32; 64] as Wire>::read(r)?;
        let placed_forest_counts = <[i32; 64] as Wire>::read(r)?;
        let known_wmd_counts = <[i32; 64] as Wire>::read(r)?;
        let cleared_bandit_camp_counts = <[i32; 64] as Wire>::read(r)?;
        let city_state_war_counts = <[i32; 64] as Wire>::read(r)?;
        let players_revived = if wire_version == 23 {
            [0; 64]
        } else {
            <[i32; 64] as Wire>::read(r)?
        };
        let fertility_added_by_droughts = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let tiles_damaged_by_droughts = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let population_lost_by_droughts = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let units_killed_by_droughts = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let fertility_added_by_floods = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let tiles_damaged_by_floods = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let population_lost_by_floods = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let units_killed_by_floods = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let fertility_added_by_storms = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let tiles_damaged_by_storms = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let population_lost_by_storms = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let units_killed_by_storms = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let fertility_added_by_volcanos = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let tiles_damaged_by_volcanos = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let population_lost_by_volcanos = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        let units_killed_by_volcanos = if wire_version == 23 {
            0
        } else {
            <i32 as Wire>::read(r)?
        };
        Ok(Self {
            wire_version,
            legacy_diplomatic_victory_points,
            max_melee_strength_trained,
            max_ranged_strength_trained,
            max_bombard_strength_trained,
            military_strength,
            military_strength_from_gold,
            air_military_strength,
            land_military_strength,
            naval_military_strength,
            barbarians_killed,
            barbarian_camps_cleared,
            tiles_pillaged,
            wmd_city_hits,
            cities_revolted,
            cities_annexed,
            total_cities_lost_to_disloyalty,
            total_cities_gained_from_disloyalty,
            total_number_of_alliances_formed,
            total_number_of_emergency_alliances_formed,
            gold_spent_levying,
            emergencies_joined,
            total_governors_appointed,
            total_governor_assignments,
            diplomatic_victory_points,
            last_turn_favor,
            current_turn_favor,
            lifetime_favor,
            science_victory_points_q8,
            science_victory_points_per_turn_q8,
            num_units_constructed,
            num_projects_advanced,
            wonders_lost_plots,
            cached_yields_q8,
            removed_feature_counts,
            national_park_counts,
            placed_forest_counts,
            known_wmd_counts,
            cleared_bandit_camp_counts,
            city_state_war_counts,
            players_revived,
            fertility_added_by_droughts,
            tiles_damaged_by_droughts,
            population_lost_by_droughts,
            units_killed_by_droughts,
            fertility_added_by_floods,
            tiles_damaged_by_floods,
            population_lost_by_floods,
            units_killed_by_floods,
            fertility_added_by_storms,
            tiles_damaged_by_storms,
            population_lost_by_storms,
            units_killed_by_storms,
            fertility_added_by_volcanos,
            tiles_damaged_by_volcanos,
            population_lost_by_volcanos,
            units_killed_by_volcanos,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        if self.legacy_diplomatic_victory_points.is_some() {
            return Err(Error::InvalidContainer);
        }
        VERSION.write(w)?;
        self.max_melee_strength_trained.write(w)?;
        self.max_ranged_strength_trained.write(w)?;
        self.max_bombard_strength_trained.write(w)?;
        self.military_strength.write(w)?;
        self.military_strength_from_gold.write(w)?;
        self.air_military_strength.write(w)?;
        self.land_military_strength.write(w)?;
        self.naval_military_strength.write(w)?;
        self.barbarians_killed.write(w)?;
        self.barbarian_camps_cleared.write(w)?;
        self.tiles_pillaged.write(w)?;
        self.wmd_city_hits.write(w)?;
        self.cities_revolted.write(w)?;
        self.cities_annexed.write(w)?;
        self.total_cities_lost_to_disloyalty.write(w)?;
        self.total_cities_gained_from_disloyalty.write(w)?;
        self.total_number_of_alliances_formed.write(w)?;
        self.total_number_of_emergency_alliances_formed.write(w)?;
        self.gold_spent_levying.write(w)?;
        self.emergencies_joined.write(w)?;
        self.total_governors_appointed.write(w)?;
        self.total_governor_assignments.write(w)?;
        self.diplomatic_victory_points.write(w)?;
        self.last_turn_favor.write(w)?;
        self.current_turn_favor.write(w)?;
        self.lifetime_favor.write(w)?;
        self.science_victory_points_q8.write(w)?;
        self.science_victory_points_per_turn_q8.write(w)?;
        self.num_units_constructed.write(w)?;
        self.num_projects_advanced.write(w)?;
        self.wonders_lost_plots.write(w)?;
        self.cached_yields_q8.write(w)?;
        self.removed_feature_counts.write(w)?;
        self.national_park_counts.write(w)?;
        self.placed_forest_counts.write(w)?;
        self.known_wmd_counts.write(w)?;
        self.cleared_bandit_camp_counts.write(w)?;
        self.city_state_war_counts.write(w)?;
        self.players_revived.write(w)?;
        self.fertility_added_by_droughts.write(w)?;
        self.tiles_damaged_by_droughts.write(w)?;
        self.population_lost_by_droughts.write(w)?;
        self.units_killed_by_droughts.write(w)?;
        self.fertility_added_by_floods.write(w)?;
        self.tiles_damaged_by_floods.write(w)?;
        self.population_lost_by_floods.write(w)?;
        self.units_killed_by_floods.write(w)?;
        self.fertility_added_by_storms.write(w)?;
        self.tiles_damaged_by_storms.write(w)?;
        self.population_lost_by_storms.write(w)?;
        self.units_killed_by_storms.write(w)?;
        self.fertility_added_by_volcanos.write(w)?;
        self.tiles_damaged_by_volcanos.write(w)?;
        self.population_lost_by_volcanos.write(w)?;
        self.units_killed_by_volcanos.write(w)?;
        Ok(())
    }
}

impl Stats {
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
    fn limits() -> Limits {
        Limits {
            max_bytes: 1 << 20,
            max_items: 10000,
        }
    }
    fn specimen() -> Stats {
        Stats {
            wire_version: VERSION,
            legacy_diplomatic_victory_points: None,
            max_melee_strength_trained: i32::MIN,
            max_ranged_strength_trained: i32::MIN,
            max_bombard_strength_trained: i32::MIN,
            military_strength: i32::MIN,
            military_strength_from_gold: i32::MIN,
            air_military_strength: i32::MIN,
            land_military_strength: i32::MIN,
            naval_military_strength: i32::MIN,
            barbarians_killed: i32::MIN,
            barbarian_camps_cleared: i32::MIN,
            tiles_pillaged: i32::MIN,
            wmd_city_hits: i32::MIN,
            cities_revolted: i32::MIN,
            cities_annexed: i32::MIN,
            total_cities_lost_to_disloyalty: i32::MIN,
            total_cities_gained_from_disloyalty: i32::MIN,
            total_number_of_alliances_formed: i32::MIN,
            total_number_of_emergency_alliances_formed: i32::MIN,
            gold_spent_levying: i32::MIN,
            emergencies_joined: i32::MIN,
            total_governors_appointed: i32::MIN,
            total_governor_assignments: i32::MIN,
            diplomatic_victory_points: TrackedValue {
                value_q8: i32::MIN,
                include_steps: false,
                min_q8: Some(-1),
                max_q8: Some(i32::MAX),
                steps: (0..2)
                    .map(|_| crate::tracked_value::Step {
                        priority: i32::MIN,
                        id: 0x80000001,
                        label: crate::map::LocalizedText {
                            base: vec![0, 0xff, b'a'],
                            arguments: vec![crate::map::LocalizedArgument::Long64(i64::MIN)],
                            named_arguments: vec![],
                        },
                        value: TrackedValue {
                            value_q8: -257,
                            include_steps: true,
                            min_q8: None,
                            max_q8: None,
                            steps: vec![],
                        },
                    })
                    .collect(),
            },
            last_turn_favor: i32::MIN,
            current_turn_favor: i32::MIN,
            lifetime_favor: i32::MIN,
            science_victory_points_q8: i32::MIN,
            science_victory_points_per_turn_q8: i32::MIN,
            num_units_constructed: vec![
                HashValue {
                    definition_hash: 0x80000001,
                    value: -1,
                },
                HashValue {
                    definition_hash: 0x80000001,
                    value: i32::MAX,
                },
            ],
            num_projects_advanced: vec![
                HashValue {
                    definition_hash: 0x80000001,
                    value: -1,
                },
                HashValue {
                    definition_hash: 0x80000001,
                    value: i32::MAX,
                },
            ],
            wonders_lost_plots: vec![-1, i32::MAX],
            cached_yields_q8: vec![
                HashValue {
                    definition_hash: 0x80000001,
                    value: -1,
                },
                HashValue {
                    definition_hash: 0x80000001,
                    value: i32::MAX,
                },
            ],
            removed_feature_counts: [i32::MIN; 64],
            national_park_counts: [i32::MIN; 64],
            placed_forest_counts: [i32::MIN; 64],
            known_wmd_counts: [i32::MIN; 64],
            cleared_bandit_camp_counts: [i32::MIN; 64],
            city_state_war_counts: [i32::MIN; 64],
            players_revived: [i32::MIN; 64],
            fertility_added_by_droughts: i32::MIN,
            tiles_damaged_by_droughts: i32::MIN,
            population_lost_by_droughts: i32::MIN,
            units_killed_by_droughts: i32::MIN,
            fertility_added_by_floods: i32::MIN,
            tiles_damaged_by_floods: i32::MIN,
            population_lost_by_floods: i32::MIN,
            units_killed_by_floods: i32::MIN,
            fertility_added_by_storms: i32::MIN,
            tiles_damaged_by_storms: i32::MIN,
            population_lost_by_storms: i32::MIN,
            units_killed_by_storms: i32::MIN,
            fertility_added_by_volcanos: i32::MIN,
            tiles_damaged_by_volcanos: i32::MIN,
            population_lost_by_volcanos: i32::MIN,
            units_killed_by_volcanos: i32::MIN,
        }
    }
    #[test]
    fn nonempty_round_trip_and_budgets() {
        let v = specimen();
        let (bytes, items) = v.encode_with_items(limits()).unwrap();
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: items,
        };
        assert_eq!(v, Stats::decode(&bytes, exact).unwrap());
        assert_eq!((bytes.clone(), items), v.encode_with_items(exact).unwrap());
        let mut carrier = bytes.clone();
        carrier.extend([0x81, 0x82]);
        assert_eq!(
            (v.clone(), bytes.len(), items),
            Stats::decode_prefix_with_items(&carrier, exact).unwrap()
        );
        for short in [
            Limits {
                max_bytes: bytes.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert!(v.encode(short).is_err());
            assert!(Stats::decode(&bytes, short).is_err());
        }
    }
    #[test]
    fn every_prefix_and_trailing() {
        let bytes = specimen().encode(limits()).unwrap();
        for n in 0..bytes.len() {
            assert!(Stats::decode(&bytes[..n], limits()).is_err(), "prefix {n}");
        }
        let mut bytes = bytes;
        bytes.push(0);
        assert_eq!(Stats::decode(&bytes, limits()), Err(Error::Trailing));
    }
    #[test]
    fn versions_and_nested_bool() {
        let mut bytes = specimen().encode(limits()).unwrap();
        bytes[0] = 26;
        assert!(matches!(
            Stats::decode(&bytes, limits()),
            Err(Error::Version { .. })
        ));
        bytes[0] = 27;
        bytes[92] = 2;
        assert!(Stats::decode(&bytes, limits()).is_err());
        bytes[92] = 1;
        bytes[100] = 2;
        assert!(Stats::decode(&bytes, limits()).is_err());
    }
    #[test]
    fn fixed_array_count_and_hostile_vector() {
        let v = specimen();
        let bytes = v.encode(limits()).unwrap();
        // The leading scalar block ends after 22 i32s.
        let tracked_len = v
            .diplomatic_victory_points
            .encode_with_items(limits())
            .unwrap()
            .0
            .len();
        let first_vector = 4 + 22 * 4 + tracked_len + 5 * 4;
        let mut bad = bytes.clone();
        bad[first_vector..first_vector + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(Stats::decode(&bad, limits()).is_err());
        let first_array = first_vector + 3 * (4 + 2 * 8) + (4 + 2 * 4);
        let mut bad = bytes;
        bad[first_array..first_array + 4].copy_from_slice(&63u32.to_le_bytes());
        assert_eq!(Stats::decode(&bad, limits()), Err(Error::InvalidContainer));
    }
    fn legacy_bytes(points: i32) -> Vec<u8> {
        let mut b = 23u32.to_le_bytes().to_vec();
        for value in 0i32..22 {
            b.extend_from_slice(&value.to_le_bytes());
        }
        b.extend_from_slice(&points.to_le_bytes());
        // Last/current favor, science points/per-turn (no lifetime-favor word).
        for value in [-7i32, 8, -257, 513] {
            b.extend_from_slice(&value.to_le_bytes());
        }
        // Unit/project hash pairs, wonders plots and cached yield pairs.
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0x80000001u32.to_le_bytes());
        b.extend_from_slice(&(-9i32).to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&(-1i32).to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        for index in 0i32..6 {
            b.extend_from_slice(&64u32.to_le_bytes());
            for value in 0i32..64 {
                b.extend_from_slice(&(index * 100 + value).to_le_bytes());
            }
        }
        b
    }
    #[test]
    fn legacy23_fields_defaults_and_exact_budget() {
        let b = legacy_bytes(-257);
        let (v, n, items) = Stats::decode_prefix_with_items(&b, limits()).unwrap();
        assert_eq!((n, items), (b.len(), 386));
        assert_eq!(v.wire_version, 23);
        assert_eq!(v.legacy_diplomatic_victory_points, Some(-257));
        assert_eq!(
            v.diplomatic_victory_points,
            TrackedValue {
                value_q8: -131584,
                include_steps: true,
                min_q8: None,
                max_q8: None,
                steps: vec![]
            }
        );
        assert_eq!(
            (v.max_melee_strength_trained, v.total_governor_assignments),
            (0, 21)
        );
        assert_eq!(
            (v.last_turn_favor, v.current_turn_favor, v.lifetime_favor),
            (-7, 8, 0)
        );
        assert_eq!(
            (
                v.science_victory_points_q8,
                v.science_victory_points_per_turn_q8
            ),
            (-257, 513)
        );
        assert_eq!(
            v.num_units_constructed[0],
            HashValue {
                definition_hash: 0x80000001,
                value: -9
            }
        );
        assert_eq!(v.wonders_lost_plots, vec![-1]);
        assert_eq!(v.city_state_war_counts[63], 563);
        assert_eq!(v.players_revived, [0; 64]);
        assert_eq!(v.fertility_added_by_droughts, 0);
        assert_eq!(v.tiles_damaged_by_droughts, 0);
        assert_eq!(v.population_lost_by_droughts, 0);
        assert_eq!(v.units_killed_by_droughts, 0);
        assert_eq!(v.fertility_added_by_floods, 0);
        assert_eq!(v.tiles_damaged_by_floods, 0);
        assert_eq!(v.population_lost_by_floods, 0);
        assert_eq!(v.units_killed_by_floods, 0);
        assert_eq!(v.fertility_added_by_storms, 0);
        assert_eq!(v.tiles_damaged_by_storms, 0);
        assert_eq!(v.population_lost_by_storms, 0);
        assert_eq!(v.units_killed_by_storms, 0);
        assert_eq!(v.fertility_added_by_volcanos, 0);
        assert_eq!(v.tiles_damaged_by_volcanos, 0);
        assert_eq!(v.population_lost_by_volcanos, 0);
        assert_eq!(v.units_killed_by_volcanos, 0);

        assert!(Stats::decode_prefix_with_items(
            &b,
            Limits {
                max_bytes: b.len(),
                max_items: 386
            }
        )
        .is_ok());
        assert_eq!(
            Stats::decode_prefix_with_items(
                &b,
                Limits {
                    max_bytes: b.len() - 1,
                    max_items: 386
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            Stats::decode_prefix_with_items(
                &b,
                Limits {
                    max_bytes: b.len(),
                    max_items: 385
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(v.encode(limits()), Err(Error::UnsupportedLegacyWrite));
    }
    #[test]
    fn legacy23_wrapping_conversion_and_refusals() {
        for (wire, raw) in [
            (i32::MIN, 0),
            (i32::MAX, -512),
            (0x40000001, 512),
            (-1, -512),
        ] {
            let v = Stats::decode(&legacy_bytes(wire), limits()).unwrap();
            assert_eq!(v.legacy_diplomatic_victory_points, Some(wire));
            assert_eq!(v.diplomatic_victory_points.value_q8, raw);
        }
        let b = legacy_bytes(3);
        for end in 0..b.len() {
            assert!(Stats::decode(&b[..end], limits()).is_err(), "prefix {end}");
        }
        let mut bad = b.clone();
        bad[..4].copy_from_slice(&24u32.to_le_bytes());
        assert_eq!(
            Stats::decode(&bad, limits()),
            Err(Error::Version {
                expected: 27,
                actual: 24
            })
        );
        let mut trailing = b;
        trailing.push(0);
        assert_eq!(Stats::decode(&trailing, limits()), Err(Error::Trailing));
    }
}
