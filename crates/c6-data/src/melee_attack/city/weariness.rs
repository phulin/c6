//! The reached major-major Unit/District ApplyWarWeariness branch. The
//! defender's owner comes from the retained combat member after a takeover.
use super::{error, Error, Generated};
use crate::{registry::LinuxDefinitionRegistry, Rules};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
pub struct Definitions {
    binding: WireBinding,
    allied: i32,
    foreign: i32,
    killed: i32,
    base: i32,
    eras: Vec<i32>,
    actions: Vec<(u32, i32)>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let p = |name| {
            crate::melee_attack::damage::integer(rules, name)
                .map_err(|e| error("weariness rules", e))
        };
        let registry: LinuxDefinitionRegistry =
            crate::unit_initialization_definitions::collection(rules, "Eras", "EraType")
                .map_err(|e| error("weariness eras", e))?;
        let table = rules
            .gameplay()
            .table("Eras")
            .ok_or_else(|| error("weariness eras", "Eras"))?;
        let eras = registry
            .entries()
            .iter()
            .map(|e| {
                table
                    .row_by_id(e.database_rowid)
                    .ok_or_else(|| error("weariness era", "row"))?
                    .integer("WarmongerPoints")
                    .map(|n| n as i32)
                    .map_err(|e| error("weariness era", e))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let table = rules
            .gameplay()
            .table("DiplomaticActions")
            .ok_or_else(|| error("weariness actions", "DiplomaticActions"))?;
        let actions = table
            .rows()
            .map(|r| {
                Ok((
                    crate::registry::linux_type_hash(
                        r.text("DiplomaticActionType")
                            .map_err(|e| error("weariness actions", e))?
                            .as_bytes(),
                    ),
                    r.integer("WarmongerPercent")
                        .map_err(|e| error("weariness actions", e))? as i32,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            allied: p("WAR_WEARINESS_PER_COMBAT_IN_ALLIED_LANDS")?,
            foreign: p("WAR_WEARINESS_PER_COMBAT_IN_FOREIGN_LANDS")?,
            killed: p("WAR_WEARINESS_PER_UNIT_KILLED")?,
            base: p("WAR_WEARINESS_WARMONGER_BASE")?,
            eras,
            actions,
        })
    }
    fn dow(&self, state: &LoadedState, owner: i32, other: i32) -> Result<i32, Error> {
        let war = state
            .game_relations()
            .map_err(|e| error("weariness relations", e))?
            .war_type(owner, other)
            .map_err(|e| error("weariness war", e))? as u32;
        // Exact war->DiplomaticAction mapping in ComputeDOWWarmongerPoints.
        let action = match war {
            0xab59e8ec => Some(0x64219cd1),
            0xd75800ed => Some(0x15b9a4db),
            0xfff75c0e => Some(0x3d16f838),
            0x09e5f2ae => Some(0x254de4ea),
            0x114b1c94 => Some(0x49ec28f1),
            0x1d3fe969 => Some(0x236dda8a),
            0x21d37fe8 => Some(0x79083542),
            0x38eb3cec => Some(0x01d50f1c),
            0x4396d725 => Some(0x1b4d9d8f),
            0x4599ae3f => Some(0x8ae1da02),
            0x73b8c966 => Some(0x1379da9e),
            0x75729ccc => Some(0xb79338fa),
            _ => None,
        };
        let percentage = action
            .and_then(|a| self.actions.iter().find(|r| r.0 == a).map(|r| r.1))
            .unwrap_or(0);
        Ok(percentage.min(player(state, owner)?.diplomacy.war_and_alliance_modifiers[6]))
    }
}
fn player(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or_else(|| error("weariness owner", owner))
}
fn delta(modifier: i32, factor: i32, combat: i32) -> i32 {
    if modifier <= -100 {
        return 0;
    }
    let product = modifier
        .wrapping_add(100)
        .wrapping_mul(factor)
        .wrapping_mul(combat);
    if product > 99 {
        (product as u32 / 100) as i32
    } else {
        0
    }
}
/// Source28d5eb0 / ChangeWarWearinessFrom2aa6e90. The logging-only publication
/// does not choose policy, update city amenities, or run the diplomacy turn.
pub fn apply(
    state: &mut LoadedState,
    definitions: &Definitions,
    maximum: &crate::formation_movement_cost::Definitions<'_>,
    levels: &[(i32, u32)],
    members: &Generated,
) -> Result<(), Error> {
    if state.binding() != &definitions.binding {
        return Err(error("weariness", "rules identity"));
    }
    let attacker = members.attacker.player();
    let defender = i32::from(members.defender.owner);
    if !(0..64).contains(&attacker) || !(0..64).contains(&defender) {
        return Err(error("weariness owners", "current player slots"));
    }
    for owner in [attacker, defender] {
        if owner == -1 {
            return Ok(());
        }
        let level = levels
            .iter()
            .find(|r| r.0 == owner)
            .ok_or_else(|| error("weariness level", owner))?
            .1;
        if level != 0x253718b0 {
            return Ok(());
        }
    }
    let (_, plot) = c6_core::movement_world::plot_at(state.world().map(), members.location)
        .map_err(|e| error("weariness plot", e))?;
    let plot_owner = crate::movement_placement_state::plot_owner(plot)
        .map_err(|e| error("weariness plot", e))?;
    let allied = plot_owner == attacker
        || (plot_owner != -1
            && state
                .game_relations()
                .map_err(|e| error("weariness relations", e))?
                .is_allied(attacker, plot_owner)
                .map_err(|e| error("weariness ally", e))?);
    let mut combat = if allied {
        definitions.allied
    } else {
        definitions.foreign
    };
    let unit = crate::melee_attack::actor(state, members.attacker)
        .map_err(|e| error("weariness attacker", e))?;
    if unit.i_damage
        >= maximum
            .maximum_damage(unit)
            .map_err(|e| error("weariness maximum", e))?
    {
        combat = combat.wrapping_add(definitions.killed);
    }
    let dow = definitions.dow(state, attacker, defender)?;
    let era = state
        .game_eras()
        .map_err(|e| error("weariness era", e))?
        .current_era()
        .ok_or_else(|| error("weariness era", "unknown current era"))?;
    let era = usize::try_from(era)
        .ok()
        .and_then(|i| definitions.eras.get(i))
        .copied()
        .ok_or_else(|| error("weariness era", "current definition index"))?;
    let a = &player(state, attacker)?
        .diplomacy
        .war_and_alliance_modifiers;
    let b = &player(state, defender)?
        .diplomacy
        .war_and_alliance_modifiers;
    let modifier = a[2]
        .wrapping_add(b[0])
        .wrapping_add(if allied { a[1] } else { 0 });
    let amount = delta(
        modifier,
        (era.wrapping_mul(dow) / 100).wrapping_add(definitions.base),
        combat,
    );
    if amount > 0 {
        let diplomacy = state
            .war_diplomacy_mut(attacker)
            .map_err(|e| error("weariness mutation", e))?;
        diplomacy.war_weariness[defender as usize] = diplomacy.war_weariness[defender as usize]
            .wrapping_add(amount)
            .max(0);
    }
    let dow = definitions.dow(state, defender, attacker)?;
    let a = &player(state, attacker)?
        .diplomacy
        .war_and_alliance_modifiers;
    let b = &player(state, defender)?
        .diplomacy
        .war_and_alliance_modifiers;
    // A District defender has no Unit pointer: native skips IsInAlliedLand and
    // unit-death additions, retaining the foreign-combat base even at its city.
    let amount = delta(
        b[2].wrapping_add(a[0]),
        definitions.base.wrapping_sub(dow),
        definitions.foreign,
    );
    if amount > 0 {
        let diplomacy = state
            .war_diplomacy_mut(defender)
            .map_err(|e| error("weariness mutation", e))?;
        diplomacy.war_weariness[attacker as usize] = diplomacy.war_weariness[attacker as usize]
            .wrapping_add(amount)
            .max(0);
    }
    Ok(())
}
