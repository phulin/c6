//! Actual Linux Unit MaxMoves/HasMoved over owned gameplay fields.
//! No Formation flag, effective-stat answer or native archive state is an input.
use crate::{
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::InputIdentities,
    loaded_components::RetainedPlayer,
    loaded_state::{Error as StateError, LoadedState},
    map::{Map, Plot, PlotFlag},
    movement_world::{DefinitionLookup, DefinitionTable},
    player_diplomacy::{Diplomacy, MovementModifier},
    unit::{PlotCoord, Unit},
};

#[derive(Debug)]
pub enum Error {
    Rules(crate::Error),
    Resources(unit_initialization_definitions::Error),
    State(StateError),
    Binding,
    Definition(&'static str),
    Scalar(&'static str),
    Player(i32),
    Map,
    District,
    Relations(c6_core::game_diplomatic_relations::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Unit movement state: {self:?}")
    }
}
impl std::error::Error for Error {}

struct Costs {
    registry: LinuxDefinitionRegistry,
    values: Vec<i32>,
}
impl Costs {
    fn from_rules(
        rules: &Rules,
        table: &'static str,
        key: &'static str,
        value: &'static str,
    ) -> Result<Self, Error> {
        let registry = collection(rules, table, key).map_err(Error::Resources)?;
        let source = rules
            .gameplay()
            .table(table)
            .ok_or(Error::Definition(table))?;
        let values = registry
            .entries()
            .iter()
            .map(|entry| {
                let row = source
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Definition(table))?;
                i32::try_from(row.integer(value).map_err(Error::Rules)?)
                    .map_err(|_| Error::Scalar(value))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { registry, values })
    }
    fn table(&self) -> Result<DefinitionTable<'_, i32>, Error> {
        DefinitionTable::new(&self.registry, &self.values).map_err(|_| Error::Definition("costs"))
    }
}

/// Immutable definitions admitted using the existing actual-resource gate.
/// Native order is retained; saved hashes never become SQL row indexes.
pub struct Definitions {
    identities: InputIdentities,
    units: LinuxDefinitionRegistry,
    base_moves: Vec<i32>,
    terrains: Costs,
    features: Costs,
    improvements: Costs,
    districts: LinuxDefinitionRegistry,
    embarked_base: i32,
    city_cost: i32,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if bound.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(Error::Resources)?;
        let units = collection(rules, "Units", "UnitType").map_err(Error::Resources)?;
        let native = bound
            .unit_definitions()
            .map_err(|_| Error::Definition("Units"))?;
        if native.lookup.count() != units.entries().len()
            || (0..native.lookup.count()).any(|i| native.lookup.type_hash(i) != units.type_hash(i))
        {
            return Err(Error::Definition("Unit native order"));
        }
        let base_moves = units
            .entries()
            .iter()
            .map(|entry| {
                let name = std::str::from_utf8(&entry.type_name)
                    .map_err(|_| Error::Definition("UnitType"))?;
                let unit = rules.unit(name).ok_or(Error::Definition("UnitType"))?;
                i32::try_from(unit.base_moves).map_err(|_| Error::Scalar("BaseMoves"))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            identities: state.binding().inputs.clone(),
            units,
            base_moves,
            terrains: Costs::from_rules(rules, "Terrains", "TerrainType", "MovementCost")?,
            features: Costs::from_rules(rules, "Features", "FeatureType", "MovementChange")?,
            improvements: Costs::from_rules(
                rules,
                "Improvements",
                "ImprovementType",
                "MovementChange",
            )?,
            districts: collection(rules, "Districts", "DistrictType").map_err(Error::Resources)?,
            embarked_base: parameter(rules, "MOVEMENT_WHILE_EMBARKED_BASE")?,
            city_cost: parameter(rules, "CITY_MOVEMENT_COST")?,
        })
    }
}
fn parameter(rules: &Rules, name: &'static str) -> Result<i32, Error> {
    let rows = rules
        .gameplay()
        .table("GlobalParameters")
        .ok_or(Error::Definition("GlobalParameters"))?;
    let mut found = rows
        .rows()
        .filter(|row| row.text("Name").ok() == Some(name));
    let row = found.next().ok_or(Error::Scalar(name))?;
    if found.next().is_some() {
        return Err(Error::Scalar(name));
    }
    match row.get("Value").map_err(Error::Rules)? {
        SqlValue::Integer(value) => i32::try_from(*value).map_err(|_| Error::Scalar(name)),
        SqlValue::Text(value) => std::str::from_utf8(value)
            .ok()
            .and_then(|s| s.parse().ok())
            .ok_or(Error::Scalar(name)),
        _ => Err(Error::Scalar(name)),
    }
}

/// Static definition inputs only. Mutable alliances are owned by LoadedState
/// and remain current across unrelated Unit queue/timer/activity writes.
pub struct Inputs<'a> {
    pub definitions: &'a Definitions,
}
fn player(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Player(owner))
}
/// Actual enum jump table from 0x2f59940. Source7 and all other values add zero.
fn timing_index(source: i32) -> Option<usize> {
    match source {
        1 => Some(2),
        2 => Some(0),
        3 => Some(1),
        4 => Some(3),
        5 => Some(4),
        6 => Some(6),
        8 => Some(5),
        _ => None,
    }
}
fn diplomatic_bonus(modifiers: &[MovementModifier], turns: &[i32; 10], current: i32) -> i32 {
    modifiers.iter().fold(0i32, |sum, modifier| {
        let Some(index) = timing_index(modifier.source) else {
            return sum;
        };
        let last = turns[index];
        if last > 0 && last.wrapping_add(modifier.turns_active) >= current {
            sum.wrapping_add(modifier.extra_moves)
        } else {
            sum
        }
    })
}
/// This reads current mutable Diplomacy timing/modifiers, not a precomputed buff.
pub fn current_diplomatic_movement(diplomacy: &Diplomacy, current_turn: i32) -> i32 {
    diplomatic_bonus(
        &diplomacy.movement_modifiers,
        &diplomacy.last_war_and_city_turns,
        current_turn,
    )
}
fn plot_at(map: &Map, location: PlotCoord) -> Result<Option<&Plot>, Error> {
    if location.x == -9999 || location.y == -9999 {
        return Ok(None);
    }
    if map.header.width <= 0 || map.header.height <= 0 {
        return Err(Error::Map);
    }
    let x = c6_core::movement_world::coord_range(location.x, map.header.width, map.header.wrap_x);
    let y = c6_core::movement_world::coord_range(location.y, map.header.height, map.header.wrap_y);
    Ok(map.plot_at(x, y))
}
fn is_city(state: &LoadedState, plot: &Plot, definitions: &Definitions) -> Result<bool, Error> {
    let Some(attrs) = &plot.city_attributes else {
        return Ok(false);
    };
    if attrs.player == -1 || attrs.district == u32::MAX {
        return Ok(false);
    }
    // CManager rejects unsupported player IDs; do not infer global validity from
    // a saved roster when the actual manager input is unavailable.
    state
        .partial_player_controls(i32::from(attrs.player))
        .map_err(Error::State)?;
    let records = &player(state, i32::from(attrs.player))?.districts.records;
    let index = (attrs.district & 0xffff) as i32;
    let Some(district) = records.entries.iter().find(|slot| slot.index == index) else {
        return Ok(false);
    };
    let Some(index) = definitions
        .districts
        .find_index(district.value.district_hash)
    else {
        return Ok(false);
    };
    Ok(definitions.districts.type_hash(index) == Some(c6_core::city_district::CITY_CENTER_HASH))
}
/// Current original Plot movement-cost projection used by Unit readiness too.
pub fn plot_movement_cost(
    state: &LoadedState,
    plot: &Plot,
    definitions: &Definitions,
) -> Result<i32, Error> {
    if definitions.identities != state.binding().inputs {
        return Err(Error::Binding);
    }
    if plot.flags.get(PlotFlag::HasCityAttributes) != plot.city_attributes.is_some() {
        return Err(Error::Map);
    }
    if is_city(state, plot, definitions)? {
        return Ok(definitions.city_cost);
    }
    let terrain = definitions.terrains.table()?;
    let Some(&mut_cost) = terrain.at(terrain
        .plot_index(plot.terrain_hash)
        .map_err(|_| Error::Definition("Terrain index"))?)
    else {
        return Ok(i32::MAX);
    };
    let mut cost = mut_cost;
    for (properties, hash) in [
        (&definitions.features, plot.feature_hash),
        (&definitions.improvements, plot.improvement_hash),
    ] {
        let table = properties.table()?;
        let index = table
            .plot_index(hash)
            .map_err(|_| Error::Definition("Plot index"))?;
        if let Some(value) = table.at(index) {
            cost = cost.wrapping_add(*value);
        }
    }
    Ok(cost)
}
fn territory_bonus(state: &LoadedState, unit: &Unit, plot: &Plot) -> Result<i32, Error> {
    let friendly = unit
        .k_movement_modifiers
        .i_friendly_territory_start_movement;
    let enemy = unit.k_movement_modifiers.i_enemy_territory_start_movement;
    if friendly == 0 && enemy == 0 {
        return Ok(0);
    }
    let owner = plot
        .city_attributes
        .as_ref()
        .map_or(-1, |c| i32::from(c.player));
    if unit.e_owner == -1 || owner == -1 {
        return Ok(0);
    }
    if owner == unit.e_owner {
        return Ok(friendly);
    }
    if player(state, owner)?.influence.suzerain == unit.e_owner {
        return Ok(friendly);
    }
    if friendly == enemy {
        return Ok(friendly);
    }
    if state
        .game_relations()
        .map_err(Error::State)?
        .is_allied(unit.e_owner, owner)
        .map_err(Error::Relations)?
    {
        Ok(friendly)
    } else {
        Ok(enemy)
    }
}

/// Original CInstance::IsEmbarked (0x2c12ca0): this activity suppresses
/// embarkation without changing the saved flag. Keep the hash value exact.
pub fn unit_is_embarked(unit: &Unit) -> bool {
    effective_embarked(unit.e_activity, unit.b_embarked)
}
fn effective_embarked(activity: i32, saved_embarked: bool) -> bool {
    activity != 0x5ff1_bbdf && saved_embarked
}

/// Original Unit.GetMaxMoves(), including signed wrapping integer arithmetic.
/// Unused pure plot branches may be elided only when their contribution is zero.
pub fn unit_max_moves(state: &LoadedState, unit: &Unit, inputs: &Inputs<'_>) -> Result<i32, Error> {
    let definitions = inputs.definitions;
    if definitions.identities != state.binding().inputs {
        return Err(Error::Binding);
    }
    let runtime = state
        .unit_runtime(unit.e_owner, unit.id)
        .map_err(Error::State)?;
    let index = definitions
        .units
        .find_index(unit.e_type_index)
        .ok_or(Error::Definition("Unit hash"))?;
    if runtime.definition_index != index as i32 {
        return Err(Error::Definition("Unit initialized index"));
    }
    let owner = player(state, unit.e_owner)?;
    let embarked = unit_is_embarked(unit);
    let base = if runtime.domain != 1 && embarked && !unit.k_combat_modifers.b_fight_while_embarked
    {
        definitions
            .embarked_base
            .wrapping_add(owner.units.embarked_movement_modifier)
    } else {
        *definitions
            .base_moves
            .get(index)
            .ok_or(Error::Definition("Unit BaseMoves"))?
    };
    let mut moves = base
        .wrapping_add(unit.k_movement_modifiers.i_extra_moves)
        .wrapping_add(current_diplomatic_movement(
            &owner.diplomacy,
            state.retained().core.current_turn,
        ));
    let modifiers = &unit.k_movement_modifiers;
    if (embarked || modifiers.i_clear_terrain_start_movement == 0)
        && modifiers.i_friendly_territory_start_movement == 0
        && modifiers.i_enemy_territory_start_movement == 0
    {
        return Ok(moves);
    }
    if let Some(plot) = plot_at(state.world().map(), unit.k_location)? {
        if plot.flags.get(PlotFlag::HasCityAttributes) != plot.city_attributes.is_some() {
            return Err(Error::Map);
        }
        if !embarked
            && modifiers.i_clear_terrain_start_movement != 0
            && plot_movement_cost(state, plot, definitions)? == 1
        {
            moves = moves.wrapping_add(modifiers.i_clear_terrain_start_movement);
        }
        moves = moves.wrapping_add(territory_bonus(state, unit, plot)?);
    }
    Ok(moves)
}
/// Original HasMoved is a strict signed raw-Q8 comparison; excess remaining
/// moves are false, and integer-to-Q8 conversion wraps exactly like x86 shl.
pub fn unit_has_moved(
    state: &LoadedState,
    unit: &Unit,
    inputs: &Inputs<'_>,
) -> Result<bool, Error> {
    Ok(unit.x_moves_remaining < unit_max_moves(state, unit, inputs)?.wrapping_shl(8))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activity_suppresses_effective_embarkation_without_changing_saved_flag() {
        for saved in [false, true] {
            assert!(!effective_embarked(0x5ff1_bbdf, saved));
            for activity in [0, -1, i32::MIN, i32::MAX, 0x5ff1_bbde, 0x5ff1_bbe0] {
                assert_eq!(effective_embarked(activity, saved), saved);
            }
        }
    }
    #[test]
    fn diplomatic_sources_inclusive_expiry_and_signed_wrapping() {
        let turns = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        for (source, index) in [(1, 2), (2, 0), (3, 1), (4, 3), (5, 4), (6, 6), (8, 5)] {
            let row = MovementModifier {
                source,
                turns_active: 2,
                extra_moves: 7,
            };
            assert_eq!(
                diplomatic_bonus(std::slice::from_ref(&row), &turns, turns[index] + 2),
                7
            );
            assert_eq!(diplomatic_bonus(&[row], &turns, turns[index] + 3), 0);
        }
        let row = |source, turns_active, extra_moves| MovementModifier {
            source,
            turns_active,
            extra_moves,
        };
        assert_eq!(
            diplomatic_bonus(&[row(7, 1000, 9), row(-1, 1000, 9)], &turns, 0),
            0
        );
        assert_eq!(diplomatic_bonus(&[row(2, i32::MAX, 3)], &turns, 0), 0);
        assert_eq!(
            diplomatic_bonus(&[row(2, 0, i32::MAX), row(2, 0, 1)], &turns, 10),
            i32::MIN
        );
        assert_eq!(diplomatic_bonus(&[row(2, 100, 9)], &[0; 10], 0), 0);
    }
}
