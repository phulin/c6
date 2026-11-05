//! Ordered Player.Units/Stats/Eras activation maintenance. Reached synchronous
//! consumers remain effects on the same state; errors retain the native prefix.
use crate::{
    loaded_actions::city_range_attack as combat, registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority, unit_initialization_definitions as init, Rules, SqlValue,
};
use c6_core::{
    loaded_components::RetainedPlayer,
    loaded_state::LoadedState,
    unit::{HashValue, Unit},
};
#[derive(Debug)]
pub enum Error<E> {
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Definitions(init::Error),
    Rules(crate::Error),
    Combat(crate::loaded_actions::Error),
    Damage(crate::formation_movement_cost::Error),
    Effect(E),
    Missing(&'static str),
}
#[derive(Clone, Copy)]
struct Era {
    hash: u32,
    chronology: i32,
}
pub struct Definitions {
    eras: Vec<Era>,
    technologies: Vec<Option<usize>>,
    civics: Vec<Option<usize>>,
    resources: LinuxDefinitionRegistry,
    fuel: Vec<Option<(usize, i32)>>,
    yields: LinuxDefinitionRegistry,
    units: LinuxDefinitionRegistry,
    ai_membership: Vec<Vec<bool>>,
    naval_ai_type: usize,
    scout_uses: LinuxDefinitionRegistry,
}
impl Definitions {
    pub(crate) fn unit_registry(&self) -> &LinuxDefinitionRegistry {
        &self.units
    }
    pub fn from_rules(
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
    ) -> Result<Self, Error<std::convert::Infallible>> {
        init::Definitions::validate_resources(rules, authority).map_err(Error::Definitions)?;
        let eras_registry =
            init::collection(rules, "Eras", "EraType").map_err(Error::Definitions)?;
        let era_table = rules
            .gameplay()
            .table("Eras")
            .ok_or(Error::Missing("Eras"))?;
        let mut eras = Vec::new();
        for e in eras_registry.entries() {
            let row = era_table
                .row_by_id(e.database_rowid)
                .ok_or(Error::Missing("Era definition"))?;
            eras.push(Era {
                hash: crate::registry::linux_type_hash(&e.type_name),
                chronology: row.integer("ChronologyIndex").map_err(Error::Rules)? as i32,
            });
        }
        let refs = |table_name,
                    key|
         -> Result<Vec<Option<usize>>, Error<std::convert::Infallible>> {
            let registry = init::collection(rules, table_name, key).map_err(Error::Definitions)?;
            let table = rules
                .gameplay()
                .table(table_name)
                .ok_or(Error::Missing("acquisition era definitions"))?;
            registry
                .entries()
                .iter()
                .map(|e| {
                    let row = table
                        .row_by_id(e.database_rowid)
                        .ok_or(Error::Missing("acquisition era row"))?;
                    match row.get("EraType").map_err(Error::Rules)? {
                        SqlValue::Null => Ok(None),
                        SqlValue::Text(name) => {
                            Ok(eras_registry.find_index(crate::registry::linux_type_hash(name)))
                        }
                        _ => Err(Error::Missing("acquisition era reference")),
                    }
                })
                .collect()
        };
        let resources =
            init::collection(rules, "Resources", "ResourceType").map_err(Error::Definitions)?;
        let units = init::collection(rules, "Units", "UnitType").map_err(Error::Definitions)?;
        let xp2 = init::collection(rules, "Units_XP2", "UnitType").map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("Units_XP2")
            .ok_or(Error::Missing("Units_XP2"))?;
        let mut fuel = Vec::new();
        for u in units.entries() {
            let mut entry = None;
            if let Some(index) = xp2.find_index(crate::registry::linux_type_hash(&u.type_name)) {
                let row = table
                    .row_by_id(xp2.entries()[index].database_rowid)
                    .ok_or(Error::Missing("Unit_XP2 row"))?;
                if let SqlValue::Text(name) =
                    row.get("ResourceMaintenanceType").map_err(Error::Rules)?
                {
                    if let Some(index) =
                        resources.find_index(crate::registry::linux_type_hash(name))
                    {
                        entry = Some((
                            index,
                            row.integer("ResourceMaintenanceAmount")
                                .map_err(Error::Rules)? as i32,
                        ));
                    }
                }
            }
            fuel.push(entry);
        }
        let ai_types =
            init::collection(rules, "UnitAiTypes", "AiType").map_err(Error::Definitions)?;
        let naval_ai_type = ai_types
            .find_index(crate::registry::linux_type_hash(b"UNITTYPE_NAVAL"))
            .ok_or(Error::Missing("UNITTYPE_NAVAL definition"))?;
        let mut ai_membership = vec![vec![false; ai_types.entries().len()]; units.entries().len()];
        for row in rules
            .gameplay()
            .table("UnitAiInfos")
            .ok_or(Error::Missing("UnitAiInfos"))?
            .rows()
        {
            if let (Some(u), Some(a)) = (
                units.find_index(crate::registry::linux_type_hash(
                    row.text("UnitType").map_err(Error::Rules)?.as_bytes(),
                )),
                ai_types.find_index(crate::registry::linux_type_hash(
                    row.text("AiType").map_err(Error::Rules)?.as_bytes(),
                )),
            ) {
                ai_membership[u][a] = true;
            }
        }
        Ok(Self {
            eras,
            units,
            ai_membership,
            naval_ai_type,
            scout_uses: init::collection(rules, "AiScoutUses", "ScoutUseType")
                .map_err(Error::Definitions)?,
            technologies: refs("Technologies", "TechnologyType")?,
            civics: refs("Civics", "CivicType")?,
            resources,
            fuel,
            yields: init::collection(rules, "Yields", "YieldType").map_err(Error::Definitions)?,
        })
    }
}
fn player<E>(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error<E>> {
    state.world().player(owner).map_err(Error::World)?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("maintenance Player"))
}
fn unit<E>(state: &LoadedState, owner: i32, id: u32) -> Result<&Unit, Error<E>> {
    state
        .world()
        .player(owner)
        .map_err(Error::World)?
        .unit(id)
        .map_err(Error::World)?
        .map(|p| p.record)
        .ok_or(Error::Missing("maintenance Unit"))
}
/// Only actual native callback boundaries occur here. Missing effects must
/// return an error, never acknowledge an unexecuted consumer.
pub trait Effects {
    type Error;
    fn report_block(
        &mut self,
        state: &mut LoadedState,
        hash: u32,
        begin: bool,
    ) -> Result<(), Self::Error>;
    fn fuel_usage(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        resource: usize,
        demand: i32,
    ) -> Result<(), Self::Error>;
    fn city_yield(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        yield_index: i32,
    ) -> Result<i32, Self::Error>;
    fn science_victory_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
    ) -> Result<(), Self::Error>;
    fn era_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        old: i32,
        new: i32,
    ) -> Result<(), Self::Error>;
}
/// Stock Linux grouping for this exact fuel block. Core's balanced in-block
/// vector only marks reporting output. Host Translate.BeginBlock3206a90 and
/// EndBlock3206ab0 return immediately unless hash==5a9a0329; this block is
/// 1153df52. There is no signal, getter, RNG or gameplay store on that branch.
/// A caller with replaced host utility methods must execute its own effects.
pub fn stock_fuel_report_block(hash: u32) -> Result<(), &'static str> {
    if hash != 0x1153df52 {
        return Err("unsupported stock reporting block");
    }
    Ok(())
}

/// Player.Units.DoTurnActivate2ba7360. Unit order is the current native chain.
/// Positive resource consumption delegates to the canonical resources/climate
/// backend; zero-demand units still reset hidden-visibility forfeiture.
pub fn units_turn_activate<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    owner: i32,
    order: &[u32],
    major: bool,
    effects: &mut E,
) -> Result<(), Error<E::Error>> {
    state
        .player_unit_maintenance_mut(owner)
        .map_err(Error::State)?
        .unit_fuel_demands
        .clear();
    for resource in defs.resources.entries() {
        state
            .player_unit_maintenance_mut(owner)
            .map_err(Error::State)?
            .unit_fuel_demands
            .push(HashValue {
                definition_hash: crate::registry::linux_type_hash(&resource.type_name),
                value: 0,
            });
    }
    for &id in order {
        state
            .unit_mut(owner, id)
            .map_err(Error::State)?
            .b_hidden_visibility_forfeited = false;
        let u = unit(state, owner, id)?;
        if u.b_ignore_resource_maintenance {
            continue;
        }
        let unit_index = defs
            .units
            .find_index(u.e_type_index)
            .ok_or(Error::Missing("current fuel maintenance unit definition"))?;
        if let Some((index, amount)) = defs.fuel.get(unit_index).copied().flatten() {
            let demands = &mut state
                .player_unit_maintenance_mut(owner)
                .map_err(Error::State)?
                .unit_fuel_demands;
            demands[index].value = demands[index].value.wrapping_add(amount);
        }
    }
    effects
        .report_block(state, 0x1153df52, true)
        .map_err(Error::Effect)?;
    if major {
        for index in 0..defs.resources.entries().len() {
            let demand = player(state, owner)?.units.unit_fuel_demands[index].value;
            if demand > 0 {
                effects
                    .fuel_usage(state, owner, index, demand)
                    .map_err(Error::Effect)?;
            }
        }
    }
    effects
        .report_block(state, 0x1153df52, false)
        .map_err(Error::Effect)
}
fn q8_mul(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 8) as i32
}
fn q8_div<E>(a: i32, b: i32) -> Result<i32, Error<E>> {
    if b == 0 {
        return Err(Error::Missing("native fixed division by zero"));
    }
    Ok(((i64::from(a) << 8) / i64::from(b)) as i32)
}
fn contribution<E>(strength: i32, damage: i32, maximum: i32) -> Result<i32, Error<E>> {
    let product = q8_mul(
        strength.wrapping_shl(8),
        maximum.wrapping_sub(damage).wrapping_shl(8),
    );
    q8_div(product, maximum.wrapping_shl(8))
}
pub fn military_strength<E>(
    state: &LoadedState,
    owner: i32,
    order: &[u32],
    domain: i32,
    combat_defs: &combat::Definitions,
    max_hit_points: i32,
) -> Result<i32, Error<E>> {
    let mut raw = 0_i32;
    for &id in order {
        if domain != -1 && state.unit_runtime(owner, id).map_err(Error::State)?.domain != domain {
            continue;
        }
        let u = unit(state, owner, id)?;
        let melee = combat::unit_combat(state, u, combat_defs).map_err(Error::Combat)?;
        let ranged = combat::unit_ranged_combat(state, u, combat_defs).map_err(Error::Combat)?;
        // Both source Getters are repeated for the winning branch. They are
        // read-only current getters; preserve the source tie choice (ranged).
        let strength = if melee > ranged {
            combat::unit_combat(state, u, combat_defs)
        } else {
            combat::unit_ranged_combat(state, u, combat_defs)
        }
        .map_err(Error::Combat)?;
        let maximum = crate::formation_movement_cost::unit_maximum_damage(u, max_hit_points)
            .map_err(Error::Damage)?;
        raw = raw.wrapping_add(contribution(strength, u.i_damage, maximum)?);
    }
    if domain == -1 {
        raw = raw.wrapping_add(q8_div(
            player::<E>(state, owner)?.treasury.gold_balance_q8,
            12_i32.wrapping_shl(8),
        )?);
    }
    Ok(raw >> 8)
}
/// Stats.DoTurnActivate2b7fc20. Every store precedes the next current query.
// Separate live unit/city orders and borrowed static combat/movement rule
// providers avoid cloning either the world or existing cached definitions.
#[allow(clippy::too_many_arguments)]
pub fn stats_turn_activate<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    owner: i32,
    order: &[u32],
    city_order: Option<&[u32]>,
    combat_defs: &combat::Definitions,
    max_hit_points: i32,
    effects: &mut E,
) -> Result<(), Error<E::Error>> {
    for domain in [-1, 1, 2, 0] {
        let value = military_strength(state, owner, order, domain, combat_defs, max_hit_points)?;
        let stats = state.player_stats_mut(owner).map_err(Error::State)?;
        match domain {
            -1 => stats.military_strength = value,
            1 => stats.air_military_strength = value,
            2 => stats.land_military_strength = value,
            _ => stats.naval_military_strength = value,
        }
    }
    let gold = (q8_div(
        player::<E::Error>(state, owner)?.treasury.gold_balance_q8,
        12_i32 << 8,
    )?) >> 8;
    state
        .player_stats_mut(owner)
        .map_err(Error::State)?
        .military_strength_from_gold = gold;
    for (index, yield_def) in defs.yields.entries().iter().enumerate() {
        let mut sum = 0_i32;
        if !player::<E::Error>(state, owner)?
            .cities
            .records
            .entries
            .is_empty()
        {
            let cities =
                city_order.ok_or(Error::Missing("current City iteration for Stats.GetYield"))?;
            for &city in cities {
                sum = sum.wrapping_add(
                    effects
                        .city_yield(state, owner, city, index as i32)
                        .map_err(Error::Effect)?,
                );
            }
        }
        let hash = crate::registry::linux_type_hash(&yield_def.type_name);
        let stats = state.player_stats_mut(owner).map_err(Error::State)?;
        let cell = stats
            .cached_yields_q8
            .iter_mut()
            .find(|p| p.definition_hash == hash)
            .ok_or(Error::Missing("initialized Stats yield cell"))?;
        cell.value = sum;
    }
    let previous = player::<E::Error>(state, owner)?.stats.current_turn_favor;
    state
        .player_stats_mut(owner)
        .map_err(Error::State)?
        .last_turn_favor = previous;
    // Congress::GetDiplomaticFavor2037a80 reads+c8: first saved Congress
    // FAutoVariable at+b8, emitted first by Serialize2a65480.
    let favor = player::<E::Error>(state, owner)?.congress.favor.player;
    state
        .player_stats_mut(owner)
        .map_err(Error::State)?
        .current_turn_favor = favor;
    let stats = state.player_stats_mut(owner).map_err(Error::State)?;
    stats.lifetime_favor = stats
        .lifetime_favor
        .wrapping_add(favor.wrapping_sub(previous).max(0));
    let points = player::<E::Error>(state, owner)?
        .stats
        .science_victory_points_per_turn_q8;
    if points != 0 {
        let stats = state.player_stats_mut(owner).map_err(Error::State)?;
        stats.science_victory_points_q8 = stats.science_victory_points_q8.wrapping_add(points);
        effects
            .science_victory_changed(state, owner)
            .map_err(Error::Effect)?;
    }
    Ok(())
}
/// Current inputs used by the reached no-city/no-region force-analysis
/// prefix. These are initialized gameplay-affecting AI fields, not requested
/// unit orders. A live producer must refresh them after relevant mutations.
pub struct ScoutInputs {
    pub binding: c6_core::core_player::initialization::WireBinding,
    pub uses_q8: Vec<i32>,
    pub fallback_q8: Option<i32>,
    pub primary_regions: Vec<u32>,
    pub secondary_regions: Vec<u32>,
    pub continent_discovery_bonus: i32,
    pub natural_wonder_discovery_bonus: i32,
}
impl Definitions {
    fn scout_value<E>(&self, inputs: &ScoutInputs, name: &[u8]) -> Result<i32, Error<E>> {
        let index = self
            .scout_uses
            .find_index(crate::registry::linux_type_hash(name));
        index
            .and_then(|i| inputs.uses_q8.get(i).copied())
            .or(inputs.fallback_q8)
            .ok_or(Error::Missing("current scout-use index/default"))
    }
}
/// AnalyzeForces214a3d1..214a7e4 updates desired counts before the actual
/// AnalyzeExplorers role loop. Nonempty regions/capital require current reveal
/// queries; they never receive a guessed percent or planning score.
pub fn prepare_explorer_demand<E>(
    state: &mut LoadedState,
    defs: &Definitions,
    owner: i32,
    major: bool,
    inputs: &ScoutInputs,
) -> Result<(), Error<E>> {
    if !major {
        return Ok(());
    }
    if inputs.binding != *state.binding() {
        return Err(Error::Missing("scout input binding"));
    }
    let land = defs.scout_value::<E>(inputs, b"DEFAULT_LAND_SCOUTS")? >> 8;
    state
        .player_ai_military_mut(owner)
        .map_err(Error::State)?
        .as_mut()
        .ok_or(Error::Missing("current AI.Military"))?
        .desired_explorers = land;
    if !inputs.primary_regions.is_empty() {
        return Err(Error::Missing("current primary region reveal queries"));
    }
    // Empty primary still performs the IndexList read and Fixed multiply.
    let _ = q8_mul(
        defs.scout_value::<E>(inputs, b"LAND_SCOUTS_PER_PRIMARY_REGION")?,
        0,
    );
    if !inputs.secondary_regions.is_empty() {
        return Err(Error::Missing("current secondary region reveal queries"));
    }
    let _ = q8_mul(
        defs.scout_value::<E>(inputs, b"LAND_SCOUTS_PER_SECONDARY_REGION")?,
        0,
    );
    let naval = defs.scout_value::<E>(inputs, b"DEFAULT_NAVAL_SCOUTS")? >> 8;
    state
        .player_ai_military_mut(owner)
        .map_err(Error::State)?
        .as_mut()
        .ok_or(Error::Missing("current AI.Military"))?
        .desired_naval_explorers = naval;
    if !player::<E>(state, owner)?.cities.records.entries.is_empty() {
        return Err(Error::Missing("current capital exploration query"));
    }
    let bonus = inputs
        .continent_discovery_bonus
        .wrapping_add(inputs.natural_wonder_discovery_bonus);
    let m = state
        .player_ai_military_mut(owner)
        .map_err(Error::State)?
        .as_mut()
        .ok_or(Error::Missing("current AI.Military"))?;
    m.desired_explorers = m.desired_explorers.wrapping_add(bonus);
    m.desired_naval_explorers = m.desired_naval_explorers.wrapping_add(bonus >> 1);
    Ok(())
}

fn military<E>(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::player_ai_military::Military, Error<E>> {
    player(state, owner)?
        .ai_military
        .as_ref()
        .ok_or(Error::Missing("current AI.Military"))
}
fn store_explorers<E>(
    state: &mut LoadedState,
    owner: i32,
    naval: bool,
    value: i32,
) -> Result<(), Error<E>> {
    let m = state
        .player_ai_military_mut(owner)
        .map_err(Error::State)?
        .as_mut()
        .ok_or(Error::Missing("current AI.Military"))?;
    if naval {
        m.current_naval_explorers = value;
    } else {
        m.current_explorers = value;
    }
    Ok(())
}
impl Definitions {
    fn has_ai<E>(&self, unit: &Unit, role: usize, initialized: bool) -> Result<bool, Error<E>> {
        ai_bit_for_type(
            &self.units,
            &self.ai_membership,
            unit.e_type_index,
            role,
            initialized,
        )
        .map_err(Error::Missing)
    }
}
fn ai_bit_for_type(
    units: &LinuxDefinitionRegistry,
    membership: &[Vec<bool>],
    unit_type: u32,
    role: usize,
    initialized: bool,
) -> Result<bool, &'static str> {
    if unit_type == u32::MAX || role == usize::MAX || !initialized {
        return Ok(false);
    }
    // Unit transport retains a type hash; live GetType supplies the native
    // definition index. Use the same admitted native-order remapping as other
    // current Unit queries, including native direct-index lookup priority.
    let index = units
        .find_index(unit_type)
        .ok_or("current unit AI membership type definition")?;
    ai_bit(membership, index as u32, role, initialized)
}
fn ai_bit(
    membership: &[Vec<bool>],
    unit_type: u32,
    role: usize,
    initialized: bool,
) -> Result<bool, &'static str> {
    if unit_type == u32::MAX || role == usize::MAX || !initialized {
        return Ok(false);
    }
    membership
        .get(unit_type as usize)
        .and_then(|r| r.get(role))
        .copied()
        .ok_or("current unit AI membership definition")
}
/// AI.Military.AnalyzeExplorers214bcd0's current no-city branch. This retains
/// the actual role/count writes; desired counts are current Military state,
/// not a native order/settlement/production policy choice. Membership bits are
/// the UnitAiInfos mapping populated at CalculateEfficiencies21bd1cf.
/// Invoke at the actual Military callback, separately from Player.Units upkeep.
pub fn analyze_explorers<E>(
    state: &mut LoadedState,
    defs: &Definitions,
    owner: i32,
    order: &[u32],
    global_ai_initialized: bool,
) -> Result<(), Error<E>> {
    store_explorers(state, owner, false, 0)?;
    store_explorers(state, owner, true, 0)?;
    for &id in order {
        let u = unit(state, owner, id)?;
        if u.e_ai_type == 2 {
            let naval = defs.has_ai(u, defs.naval_ai_type, global_ai_initialized)?;
            let m = military(state, owner)?;
            let value = if naval {
                m.current_naval_explorers
            } else {
                m.current_explorers
            };
            store_explorers(state, owner, naval, value.wrapping_add(1))?;
        }
    }
    if !player::<E>(state, owner)?.cities.records.entries.is_empty() {
        return Err(Error::Missing("current City build-front explorer count"));
    }
    // Land growth has two passes: preserve assigned defenders in the first,
    // then admit combat-capable defenders only if demand is still unmet.
    for pass in 0..2 {
        if military::<E>(state, owner)?.current_explorers
            >= military::<E>(state, owner)?.desired_explorers
        {
            break;
        }
        for &id in order {
            let u = unit(state, owner, id)?;
            if u.ai_controller != u32::MAX || u.e_ai_type == 2 || (pass == 0 && u.e_ai_type == 4) {
                continue;
            }
            if !defs.has_ai(u, if pass == 0 { 2 } else { 3 }, global_ai_initialized)? {
                continue;
            }
            state.unit_mut(owner, id).map_err(Error::State)?.e_ai_type = 2;
            let count = military::<E>(state, owner)?
                .current_explorers
                .wrapping_add(1);
            store_explorers(state, owner, false, count)?;
            if count >= military::<E>(state, owner)?.desired_explorers {
                break;
            }
        }
    }
    if military::<E>(state, owner)?.current_explorers
        > military::<E>(state, owner)?.desired_explorers
    {
        for &id in order {
            let u = unit(state, owner, id)?;
            if u.ai_controller != u32::MAX
                || u.e_ai_type != 2
                || !defs.has_ai(u, 3, global_ai_initialized)?
            {
                continue;
            }
            state.unit_mut(owner, id).map_err(Error::State)?.e_ai_type = 3;
            let count = military::<E>(state, owner)?
                .current_explorers
                .wrapping_sub(1);
            store_explorers(state, owner, false, count)?;
            if count <= military::<E>(state, owner)?.desired_explorers {
                break;
            }
        }
    }
    let m = military::<E>(state, owner)?;
    if m.current_naval_explorers < m.desired_naval_explorers {
        for &id in order {
            let u = unit(state, owner, id)?;
            if u.ai_controller != u32::MAX
                || u.e_ai_type == 2
                || !defs.has_ai(u, 2, global_ai_initialized)?
                || !defs.has_ai(u, defs.naval_ai_type, global_ai_initialized)?
            {
                continue;
            }
            state.unit_mut(owner, id).map_err(Error::State)?.e_ai_type = 2;
            let count = military::<E>(state, owner)?
                .current_naval_explorers
                .wrapping_add(1);
            store_explorers(state, owner, true, count)?;
            if count >= military::<E>(state, owner)?.desired_naval_explorers {
                break;
            }
        }
    } else if m.current_naval_explorers > m.desired_naval_explorers {
        for &id in order {
            let u = unit(state, owner, id)?;
            if u.ai_controller != u32::MAX
                || u.e_ai_type != 2
                || !defs.has_ai(u, 3, global_ai_initialized)?
                || !defs.has_ai(u, defs.naval_ai_type, global_ai_initialized)?
            {
                continue;
            }
            state.unit_mut(owner, id).map_err(Error::State)?.e_ai_type = 3;
            let count = military::<E>(state, owner)?
                .current_naval_explorers
                .wrapping_sub(1);
            store_explorers(state, owner, true, count)?;
            if count <= military::<E>(state, owner)?.desired_naval_explorers {
                break;
            }
        }
    }
    Ok(())
}

fn first_era(eras: &[Era]) -> Option<usize> {
    let mut best = None;
    let mut chronology = i32::MAX;
    for (i, era) in eras.iter().enumerate() {
        if era.chronology < chronology {
            chronology = era.chronology;
            best = Some(i);
        }
    }
    best
}
pub fn eras_turn_activate<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    owner: i32,
    effects: &mut E,
) -> Result<(), Error<E::Error>> {
    let first = first_era(&defs.eras).ok_or(Error::Missing("first Era"))?;
    let mut chosen = [first; 2];
    for (kind, mapping) in [&defs.technologies, &defs.civics].into_iter().enumerate() {
        let flags = if kind == 0 {
            state
                .world()
                .player(owner)
                .map_err(Error::World)?
                .technologies
                .as_ref()
        } else {
            state
                .world()
                .player(owner)
                .map_err(Error::World)?
                .civics
                .as_ref()
        }
        .ok_or(Error::Missing("current acquired definitions"))?;
        let mut chronology = i32::MIN;
        for (index, era) in mapping.iter().enumerate() {
            if !*flags
                .get(index)
                .ok_or(Error::Missing("complete current acquired flags"))?
            {
                continue;
            }
            if let Some(era) = era {
                if defs.eras[*era].chronology > chronology {
                    chronology = defs.eras[*era].chronology;
                    chosen[kind] = *era;
                }
            }
        }
    }
    let selected = if defs.eras[chosen[0]].chronology < defs.eras[chosen[1]].chronology {
        chosen[1]
    } else {
        chosen[0]
    };
    let old_hash = player::<E::Error>(state, owner)?.eras.era_hash;
    let selected_hash = defs.eras[selected].hash;
    if old_hash != selected_hash {
        state.player_eras_mut(owner).map_err(Error::State)?.era_hash = selected_hash;
        let old = defs
            .eras
            .iter()
            .position(|e| e.hash == old_hash)
            .map_or(-1, |i| i as i32);
        effects
            .era_changed(state, owner, old, selected as i32)
            .map_err(Error::Effect)?;
    }
    let global = state
        .game_eras()
        .map_err(|_| Error::Missing("current Game.Eras"))?
        .current_era()
        .ok_or(Error::Missing("current game era"))?;
    let hash = if global == -1 {
        u32::MAX
    } else {
        defs.eras
            .get(global as u32 as usize)
            .ok_or(Error::Missing("current game era definition"))?
            .hash
    };
    state
        .player_eras_mut(owner)
        .map_err(Error::State)?
        .game_era_hash = hash;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn health_q8_rounding_and_aggregate_boundary() {
        assert_eq!(contribution::<()>(20, 0, 100).unwrap(), 20 << 8);
        assert_eq!(contribution::<()>(21, 1, 100).unwrap(), 5322);
        assert_eq!(contribution::<()>(20, 101, 100).unwrap(), -51);
        assert!(contribution::<()>(20, 0, 0).is_err());
    }
    #[test]
    fn era_chronology_ties_keep_first_native_entry() {
        assert_eq!(
            first_era(&[
                Era {
                    hash: 3,
                    chronology: 5
                },
                Era {
                    hash: 9,
                    chronology: -2
                },
                Era {
                    hash: 1,
                    chronology: -2
                }
            ]),
            Some(1)
        );
    }
    #[test]
    fn gold_fraction_is_added_before_integer_projection() {
        let gold = q8_div::<()>(11 << 8, 12 << 8).unwrap();
        assert_eq!(
            (contribution::<()>(1, 50, 100).unwrap().wrapping_add(gold)) >> 8,
            1
        );
        assert_eq!(gold >> 8, 0);
    }
    #[test]
    fn membership_resolves_saved_unit_hash_in_native_definition_order() {
        use crate::{
            registry::{linux_type_hash, DefinitionEntry},
            RulesRowId,
        };
        let units = LinuxDefinitionRegistry::from_native_order(vec![
            DefinitionEntry {
                database_rowid: RulesRowId(90),
                type_name: b"UNIT_SETTLER".to_vec(),
            },
            DefinitionEntry {
                database_rowid: RulesRowId(2),
                type_name: b"UNIT_WARRIOR".to_vec(),
            },
        ]);
        let membership = vec![vec![false, false, false], vec![false, false, true]];
        assert_eq!(
            ai_bit_for_type(
                &units,
                &membership,
                linux_type_hash(b"UNIT_WARRIOR"),
                2,
                true
            ),
            Ok(true)
        );
        assert_eq!(
            ai_bit_for_type(
                &units,
                &membership,
                linux_type_hash(b"UNIT_SETTLER"),
                2,
                true
            ),
            Ok(false)
        );
        assert_eq!(ai_bit_for_type(&units, &membership, 1, 2, true), Ok(true));
        assert!(ai_bit_for_type(
            &units,
            &membership,
            linux_type_hash(b"UNIT_MISSING"),
            2,
            true
        )
        .is_err());
        assert_eq!(
            ai_bit_for_type(
                &units,
                &membership,
                linux_type_hash(b"UNIT_MISSING"),
                2,
                false
            ),
            Ok(false)
        );
    }
    #[test]
    fn membership_guards_skip_uninitialized_arrays_but_unknown_initialized_is_error() {
        assert_eq!(ai_bit(&[], 99, 2, false), Ok(false));
        assert_eq!(ai_bit(&[], u32::MAX, 2, true), Ok(false));
        assert!(ai_bit(&[], 99, 2, true).is_err());
        assert_eq!(ai_bit(&[vec![false, false, true]], 0, 2, true), Ok(true));
    }
}
