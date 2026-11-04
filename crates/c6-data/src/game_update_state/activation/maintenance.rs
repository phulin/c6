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
}
impl Definitions {
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
        Ok(Self {
            eras,
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
        if let Some((index, amount)) = defs.fuel.get(u.e_type_index as usize).copied().flatten() {
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
    damage_defs: &crate::formation_movement_cost::Definitions<'_>,
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
        let maximum = damage_defs.maximum_damage(u).map_err(Error::Damage)?;
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
    damage_defs: &crate::formation_movement_cost::Definitions<'_>,
    effects: &mut E,
) -> Result<(), Error<E::Error>> {
    for domain in [-1, 1, 2, 0] {
        let value = military_strength(state, owner, order, domain, combat_defs, damage_defs)?;
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
}
