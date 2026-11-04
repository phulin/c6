//! Linux IsBuildingThemedCorrectly and CheckForThemedBuilding.
//! Uses the actual city slots, global GreatWorks and runtime rule references.
//! All reached GameSignals+0x240 handlers remain required synchronous effects.
use super::{Building, Context as RelicContext, GreatWork};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlayerTheme {
    /// Actual runtime enum, after resolving the Culture wire hash if imported.
    pub auto_building: i32,
    pub buildings_with_x_slots: i32,
    pub wonders_with_x_slots: i32,
}
impl PlayerTheme {
    pub fn from_culture(culture: &crate::player_culture::Culture, auto_building: i32) -> Self {
        Self {
            auto_building,
            buildings_with_x_slots: culture.auto_themed_buildings_with_x_slots,
            wonders_with_x_slots: culture.auto_themed_wonders_with_x_slots,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GreatWorkRule {
    pub num_slots: i32,
    pub tourism_multiplier: i32,
    pub yield_multiplier: i32,
    pub unique_person: bool,
    pub unique_civs: bool,
    pub same_object: bool,
    pub same_eras: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildingDefinition {
    pub wonder: bool,
    /// Actual native reference collection, in order, including all entries.
    pub great_work_rules: Vec<GreatWorkRule>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkMetadata {
    /// Null pointers differ from an existing empty C string.
    pub great_person_individual: Option<Vec<u8>>,
    /// Resolved actual Era reference's collection index, or captured null.
    pub era: Option<i32>,
}

/// Only runtime input lookups and actual signal dispatch remain providers;
/// theming decisions and all slot/work comparisons are executed below.
pub trait Context: RelicContext {
    /// Exact DefinitionCollectionWithTypeIndex<Building>::Find(unsigned int):
    /// first index if argument<count, otherwise hash lookup; None is observed
    /// null/absent, not an unavailable rule collection.
    fn theme_building_definition(
        &self,
        id: i32,
    ) -> Result<Option<&BuildingDefinition>, Self::Error>;
    fn theme_player(&mut self, owner: i32) -> Result<PlayerTheme, Self::Error>;
    fn theme_work_metadata(&self, work_type: i32) -> Result<&WorkMetadata, Self::Error>;
    /// Resolve City ComponentID through CManager::EditCity and synchronously
    /// dispatch GameSignals+0x240 with (resolved city pointer, building enum).
    /// Must execute every reached gameplay/RNG/script handler; no empty default.
    fn signal_themed_building(
        &mut self,
        city: Self::City,
        building: Building,
    ) -> Result<(), Self::Error>;
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error<E> {
    Context(E),
    MissingGreatWork(i32),
    MissingWorkDefinition(i32),
    EmptyRequiredSlotList,
}

fn work<C: Context>(c: &C, index: i32) -> Result<&GreatWork, Error<C::Error>> {
    c.great_works()
        .get(index as usize)
        .ok_or(Error::MissingGreatWork(index))
}
fn cstring(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|&v| v == 0).unwrap_or(bytes.len())]
}

pub fn is_building_themed_correctly<C: Context>(
    c: &mut C,
    city: C::City,
    building: Building,
) -> Result<bool, Error<C::Error>> {
    use Error::Context;
    // This surprising native default is explicit target behavior.
    let Some(definition) = c
        .theme_building_definition(building.index)
        .map_err(Context)?
        .cloned()
    else {
        return Ok(true);
    };
    let owner = c.city_owner(city).map_err(Context)?;
    let player = c.theme_player(owner).map_err(Context)?;
    let buildings = c.buildings(city).map_err(Context)?;
    let first = buildings
        .great_work_buildings
        .iter()
        .find(|b| b.building_hash == building.hash);
    if player.auto_building != -1 && player.auto_building == building.index {
        if let Some(record) = first {
            return Ok(record.slots.iter().all(|s| s.great_work_index != -1));
        }
    }
    let threshold = if definition.wonder {
        player.wonders_with_x_slots
    } else {
        player.buildings_with_x_slots
    };
    let extras = buildings
        .extra_great_works
        .iter()
        .filter(|e| e.building_hash == building.hash)
        .fold(0i32, |total, e| total.wrapping_add(e.count));
    let num_slots = definition
        .great_work_rules
        .iter()
        .fold(extras, |total, r| total.wrapping_add(r.num_slots));
    // No invented >0 threshold guard: compare the actual signed values.
    if num_slots >= threshold {
        if let Some(record) = first {
            return Ok(record.slots.iter().all(|s| s.great_work_index != -1));
        }
    }
    if definition.great_work_rules.len() != 1 {
        return Ok(false);
    }
    let rule = definition.great_work_rules[0];
    if rule.tourism_multiplier <= 0 && rule.yield_multiplier <= 0 {
        return Ok(false);
    }
    let mut indices = Vec::new();
    for record in &buildings.great_work_buildings {
        if record.building_hash != building.hash {
            continue;
        }
        for slot in &record.slots {
            if slot.great_work_index == -1 {
                return Ok(false);
            }
            indices.push(slot.great_work_index);
        }
    }
    if rule.unique_person && indices.len() > 1 {
        for (i, &first) in indices.iter().enumerate().take(indices.len() - 1) {
            let first = work(c, first)?;
            if first.work_type == -1 {
                continue;
            }
            let Some(name) = &c
                .theme_work_metadata(first.work_type)
                .map_err(Context)?
                .great_person_individual
            else {
                continue;
            };
            for &second in &indices[i + 1..] {
                let second = work(c, second)?;
                if second.work_type == -1 {
                    continue;
                }
                if let Some(other) = &c
                    .theme_work_metadata(second.work_type)
                    .map_err(Context)?
                    .great_person_individual
                {
                    if cstring(name) == cstring(other) {
                        return Ok(false);
                    }
                }
            }
        }
    }
    if rule.unique_civs && indices.len() > 1 {
        for (i, &first) in indices.iter().enumerate().take(indices.len() - 1) {
            let player = work(c, first)?.player;
            for &second in &indices[i + 1..] {
                let other = work(c, second)?.player;
                if other != -1 && player == other {
                    return Ok(false);
                }
            }
        }
    }
    if rule.same_object {
        let first = *indices.first().ok_or(Error::EmptyRequiredSlotList)?;
        let first = work(c, first)?.work_type;
        // Native skips the entire same-object comparison if first kind=-1.
        if first != -1 {
            let object = c
                .rules()
                .works
                .iter()
                .find(|d| d.index == first)
                .ok_or(Error::MissingWorkDefinition(first))?
                .object_type;
            for &next in &indices[1..] {
                let next = work(c, next)?.work_type;
                if next == -1 {
                    return Ok(false);
                }
                let other = c
                    .rules()
                    .works
                    .iter()
                    .find(|d| d.index == next)
                    .ok_or(Error::MissingWorkDefinition(next))?
                    .object_type;
                if object != other {
                    return Ok(false);
                }
            }
        }
    }
    if rule.same_eras {
        let first = *indices.first().ok_or(Error::EmptyRequiredSlotList)?;
        let first = work(c, first)?.work_type;
        let era = c
            .theme_work_metadata(first)
            .map_err(Context)?
            .era
            .unwrap_or(-1);
        for &next in &indices[1..] {
            let next = work(c, next)?.work_type;
            let other = c
                .theme_work_metadata(next)
                .map_err(Context)?
                .era
                .unwrap_or(-1);
            if era == -1 || era != other {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

pub fn check_for_themed_building<C: Context>(
    c: &mut C,
    city: C::City,
    building: Building,
) -> Result<(), Error<C::Error>> {
    if is_building_themed_correctly(c, city, building)? {
        c.signal_themed_building(city, building)
            .map_err(Error::Context)?;
    }
    Ok(())
}

/// Inputs/effect provider for the native BoostManager themed-building consumer.
/// The active signal registration supplies its real Boost definition/identity.
pub trait BoostContext: Context {
    type Boost: Copy;
    fn theme_building_order(&self) -> Result<&[Building], Self::Error>;
    fn theme_boost_num_items(&self, boost: Self::Boost) -> Result<i32, Self::Error>;
    /// Complete Boost::Manager::TriggerBoost (and reached tech/civic state,
    /// callbacks/reporting/RNG), never a flag-only substitute.
    fn trigger_theme_boost(&mut self, boost: Self::Boost, player: i32) -> Result<(), Self::Error>;
}

/// Player::Stats::GetNumThemedBuildings: city iteration outside complete native
/// Building-definition iteration; HasBuilding gates each exact predicate.
pub fn count_themed_buildings<C: BoostContext>(
    c: &mut C,
    player: i32,
) -> Result<i32, Error<C::Error>> {
    use Error::Context;
    let mut count = 0i32;
    let mut index = 0;
    while index < c.city_count(player).map_err(Context)? {
        let city = c.city_at(player, index).map_err(Context)?;
        let definitions = c.theme_building_order().map_err(Context)?.to_vec();
        for building in definitions {
            if building.index == -1 {
                continue;
            }
            if c.building_location(city, building).map_err(Context)? >= 0
                && is_building_themed_correctly(c, city, building)?
            {
                count = count.wrapping_add(1);
            }
        }
        index += 1;
    }
    Ok(count)
}

/// Boost::Manager::CheckNumThemedBuildings. The incoming building argument is
/// unused: the source counts all themed buildings of the current city owner.
pub fn check_num_themed_buildings<C: BoostContext>(
    c: &mut C,
    city: C::City,
    boost: C::Boost,
) -> Result<(), Error<C::Error>> {
    let player = c.city_owner(city).map_err(Error::Context)?;
    let count = count_themed_buildings(c, player)?;
    let required = c.theme_boost_num_items(boost).map_err(Error::Context)?;
    if count >= required {
        c.trigger_theme_boost(boost, player)
            .map_err(Error::Context)?;
    }
    Ok(())
}
